package fx

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"os/user"
	"path/filepath"
	"strconv"
	"time"

	"github.com/tsiemens/acb/date"
	decimal "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/log"
	"github.com/tsiemens/acb/util"
)

const (
	cadUsdNoonObs    = "IEXE0101"
	cadUsdIndObs     = "FXCADUSD"
	cadUsdJsonUrlFmt = "https://www.bankofcanada.ca/valet/observations/%s/json?start_date=%d-01-01&end_date=%d-12-31"

	lineBufSize     = 100
	csvTimeFormat   = "2006-01-02"
	csvPrintTimeFmt = "%d-%02d-%02d"
)

type ValetJsonFx struct {
	ValStr string `json:"v"`
}

func (v ValetJsonFx) Val() (float64, error) {
	if v.ValStr == "" {
		return 0.0, nil
	}
	return strconv.ParseFloat(v.ValStr, 64)
}

type ValetJsonObs struct {
	Date       string      `json:"d"`
	UsdCadNoon ValetJsonFx `json:"IEXE0101"`
	UsdCad     ValetJsonFx `json:"FXCADUSD"`
}

type ValetJsonRoot struct {
	Observations []ValetJsonObs `json:"observations"`
}

func getJsonUrl(year uint32) string {
	var obs string
	if year >= 2017 {
		obs = cadUsdIndObs
	} else {
		obs = cadUsdNoonObs
	}
	return fmt.Sprintf(cadUsdJsonUrlFmt, obs, year, year)
}

type RemoteRateLoader interface {
	GetRemoteUsdCadRates(year uint32) ([]DailyRate, error)
}

type JsonRemoteRateLoader struct {
	ErrPrinter log.ErrorPrinter
}

// Verify that *JsonRemoteRateLoader implements RemoteRateLoader
var _ RemoteRateLoader = (*JsonRemoteRateLoader)(nil)

func (l *JsonRemoteRateLoader) GetRemoteUsdCadRates(year uint32) ([]DailyRate, error) {
	fmt.Fprintf(os.Stderr, "Fetching USD/CAD exchange rates for %d\n", year)
	url := getJsonUrl(year)
	log.Fverbosef(os.Stderr, "Getting %s\n", url)
	resp, err := http.Get(url)
	if err != nil {
		return nil, fmt.Errorf("Error getting CAD USD rates: %v", err)
	} else if resp.StatusCode != 200 {
		return nil, fmt.Errorf("Error status: %s", resp.Status)
	}

	var theJson ValetJsonRoot
	dcdr := json.NewDecoder(resp.Body)
	err = dcdr.Decode(&theJson)
	if err != nil {
		return nil, err
	}

	rates := make([]DailyRate, 0, len(theJson.Observations))
	for _, obs := range theJson.Observations {
		date, err := date.Parse(csvTimeFormat, obs.Date)
		if err != nil {
			l.ErrPrinter.Ln("Unable to parse date:", err)
			continue
		}

		var dRate DailyRate
		usdCadNoonVal, err := obs.UsdCadNoon.Val()
		if err != nil {
			l.ErrPrinter.Ln("Failed to parse USDCAD Noon rate for", date, ":", obs.UsdCadNoon.ValStr)
			continue
		}

		if usdCadNoonVal != 0.0 {
			dRate = DailyRate{date, decimal.NewFromFloat(usdCadNoonVal)}
		} else {
			usdCadVal, err := obs.UsdCad.Val()
			if err != nil {
				l.ErrPrinter.Ln("Failed to parse USDCAD rate for", date, ":", obs.UsdCad.ValStr)
				continue
			}
			dRate = DailyRate{date, decimal.NewFromInt(1).Div(decimal.NewFromFloat(usdCadVal))}
		}
		rates = append(rates, dRate)
	}
	return rates, nil
}

type RatesCache interface {
	WriteRates(year uint32, rates []DailyRate) error
	GetUsdCadRates(year uint32) ([]DailyRate, error)
}

type MemRatesCacheAccessor struct {
	RatesByYear map[uint32][]DailyRate
}

func NewMemRatesCacheAccessor() *MemRatesCacheAccessor {
	return &MemRatesCacheAccessor{RatesByYear: make(map[uint32][]DailyRate)}
}

func (c *MemRatesCacheAccessor) WriteRates(year uint32, rates []DailyRate) error {
	c.RatesByYear[year] = rates
	return nil
}

func (c *MemRatesCacheAccessor) GetUsdCadRates(year uint32) ([]DailyRate, error) {
	rates, ok := c.RatesByYear[year]
	if !ok {
		return nil, nil
	}
	return rates, nil
}

type CsvRatesCache struct {
	ErrPrinter log.ErrorPrinter
}

func (c *CsvRatesCache) WriteRates(year uint32, rates []DailyRate) error {
	return WriteRatesToCsv(year, rates)
}

func (c *CsvRatesCache) GetUsdCadRates(year uint32) ([]DailyRate, error) {
	file, err := ratesCsvFile(year, false)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	return c.getRatesFromCsv(file)
}

// Fills in gaps in daily rates (for a single year) with zero
// If today is in the same year as the rates, will fill up to yesterday, with the
// assumption that today's rate wouldn't yet be published.
func FillInUnknownDayRates(rates []DailyRate, year uint32) []DailyRate {
	filledRates := make([]DailyRate, 0, int(float32(len(rates))*(7.0/5.0)))
	dateToFill := date.New(year, time.January, 1)
	for _, rate := range rates {
		for dateToFill.Before(rate.Date) {
			filledRates = append(filledRates, DailyRate{dateToFill, decimal.Zero})
			dateToFill = dateToFill.AddDays(1)
		}
		filledRates = append(filledRates, rate)
		dateToFill = dateToFill.AddDays(1)
	}

	today := date.Today()
	for dateToFill.Before(today) && uint32(dateToFill.Year()) == year {
		filledRates = append(filledRates, DailyRate{dateToFill, decimal.Zero})
		dateToFill = dateToFill.AddDays(1)
	}
	return filledRates
}

func (c *CsvRatesCache) getRatesFromCsv(r io.Reader) ([]DailyRate, error) {
	csvR := csv.NewReader(r)
	csvR.FieldsPerRecord = 2
	records, err := csvR.ReadAll()
	if err != nil {
		return nil, err
	}

	rates := make([]DailyRate, 0, len(records))

	for _, record := range records {
		date, err := date.Parse(csvTimeFormat, record[0])
		if err != nil {
			c.ErrPrinter.Ln("Unable to parse date:", err)
			continue
		}
		rate, err := decimal.NewFromString(record[1])
		if err != nil {
			c.ErrPrinter.Ln("Unable to parse rate:", err)
			continue
		}

		dRate := DailyRate{date, rate}
		rates = append(rates, dRate)
	}

	return rates, nil
}

func HomeDirFile(fname string) (string, error) {
	const dir = ".acb"
	usr, err := user.Current()
	if err != nil {
		return "", err
	}
	dirPath := filepath.Join(usr.HomeDir, dir)
	os.MkdirAll(dirPath, 0700)
	return filepath.Join(dirPath, url.QueryEscape(fname)), err
}

func ratesCsvFile(year uint32, write bool) (*os.File, error) {
	preFname := fmt.Sprintf("rates-%d.csv", year)
	fname, err := HomeDirFile(preFname)
	if err != nil {
		return nil, err
	}
	if write {
		return os.OpenFile(fname, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, os.ModePerm)
	}
	return os.Open(fname)
}

func rateDateCsvStr(r DailyRate) string {
	year, month, day := r.Date.Parts()
	return fmt.Sprintf(csvPrintTimeFmt, year, month, day)
}

func WriteRatesToCsv(year uint32, rates []DailyRate) (err error) {
	err = nil
	file, err := ratesCsvFile(year, true)
	if err != nil {
		return
	}
	defer func() {
		cerr := file.Close()
		if err != nil {
			err = cerr
		}
	}()

	csvW := csv.NewWriter(file)
	for _, rate := range rates {
		row := []string{rateDateCsvStr(rate), fmt.Sprintf("%f", rate.ForeignToLocalRate)}
		err = csvW.Write(row)
		if err != nil {
			return
		}
	}
	csvW.Flush()
	return
}

type RateLoader struct {
	YearRates        map[uint32]map[date.Date]DailyRate
	ForceDownload    bool
	Cache            RatesCache
	RemoteLoader     RemoteRateLoader
	FreshLoadedYears map[uint32]bool
	ErrPrinter       log.ErrorPrinter
}

func NewRateLoader(
	forceDownload bool, ratesCache RatesCache, errPrinter log.ErrorPrinter) *RateLoader {
	return &RateLoader{
		YearRates:        make(map[uint32]map[date.Date]DailyRate),
		ForceDownload:    forceDownload,
		Cache:            ratesCache,
		RemoteLoader:     &JsonRemoteRateLoader{errPrinter},
		FreshLoadedYears: make(map[uint32]bool),
		ErrPrinter:       errPrinter,
	}
}

func (cr *RateLoader) GetRemoteUsdCadRatesJson(year uint32, ratesCache RatesCache) ([]DailyRate, error) {
	rates, err := cr.RemoteLoader.GetRemoteUsdCadRates(year)
	if err != nil {
		return nil, err
	}
	rates = FillInUnknownDayRates(rates, year)

	cr.FreshLoadedYears[year] = true
	err = ratesCache.WriteRates(year, rates)
	if err != nil {
		cr.ErrPrinter.Ln("Failed to update exchange rate cache:", err)
	}
	return rates, nil
}

func makeDateToRateMap(rates []DailyRate) map[date.Date]DailyRate {
	ratesMap := make(map[date.Date]DailyRate)
	for _, rate := range rates {
		ratesMap[rate.Date] = rate
	}
	return ratesMap
}

/* Loads exchange rates for year from cache or from remote web API.
 * @year - year to load.
 * @targetDay - The target date we're loading for.
 *
 * Will use the cache if we are not force downloading, if we already downloaded
 * in this process run, or if `targetDay` has a defined value in the cache
 * (even if it is defined as zero).
 * Using `targetDay` for cache invalidation allows us to avoid invalidating the cache if
 * there are no new transactions.
 */
func (cr *RateLoader) fetchUsdCadRatesForDateYear(
	targetDay date.Date) (map[date.Date]DailyRate, error) {
	year := uint32(targetDay.Year())
	var ratesMap map[date.Date]DailyRate

	if !cr.ForceDownload {
		// Try the cache
		rates, err := cr.Cache.GetUsdCadRates(year)
		_, ratesAreFresh := cr.FreshLoadedYears[year]
		if err != nil {
			if ratesAreFresh {
				// We already loaded this year from remote during this process.
				// Something is wrong if we tried to access it via the cache again and it
				// failed (since we are not allowed to make the same request again).
				return nil, err
			}
			cr.ErrPrinter.Ln("Could not load cached exchange rates:", err)
		}
		ratesMap = makeDateToRateMap(rates)
		if !ratesAreFresh {
			// Check for cache invalidation.
			if _, ok := ratesMap[targetDay]; ok {
				return ratesMap, nil
			}
		} else {
			return ratesMap, nil
		}
	}

	rates, err := cr.GetRemoteUsdCadRatesJson(year, cr.Cache)
	if err != nil {
		return nil, err
	}
	return makeDateToRateMap(rates), nil
}

/*
TL;DR official recommendation appears to be to get the "active" rate on the trade
day, which is the last known rate (we can'tradeDate see the future, obviously).

As per the CRA's interpretation of Section 261 (1.4)
https://www.canada.ca/en/revenue-agency/services/tax/technical-information/income-tax/income-tax-folios-index/series-5-international-residency/series-5-international-residency-folio-4-foreign-currency/income-tax-folio-s5-f4-c1-income-tax-reporting-currency.html

For a particular day after February 28, 2017, the relevant spot rate is to be used to
convert an amount from one currency to another, where one of the currencies is
Canadian currency is the rate quoted by the Bank of Canada on that day. If the Bank
of Canada ordinarily quotes such a rate, but no rate is quoted for that particular
day, then the closest preceding day for which such a rate is quoted should be used.
If the particular day, or closest preceding day, of conversion is before
March 1, 2017, the Bank of Canada noon rate should be used.

NOTE: This function should NOT be called for today if the rate is not yet knowable.
*/
func (cr *RateLoader) findUsdCadPrecedingRelevantSpotRate(
	tradeDate date.Date, foundRate DailyRate) (DailyRate, error) {

	const errFmt = "%s. As per Section 261(1) of the Income Tax Act, the exchange rate " +
		"from the preceding day for which such a rate is quoted should be " +
		"used if no rate is quoted on the day the trade."

	util.Assertf(foundRate == DailyRate{tradeDate, decimal.Zero},
		"findUsdCadPrecedingRelevantSpotRate: rate for %s must be explicitly "+
			"marked as 'markets closed' with a rate of zero\n",
		tradeDate)

	precedingDate := tradeDate
	// Limit to 7 days look-back. This is arbitrarily chosen as a large-enough value
	// (unless the markets close for more than a week due to an apocalypse)
	for i := 0; i < 7; i++ {
		precedingDate = precedingDate.AddDays(-1)
		rate, err := cr.GetExactUsdCadRate(precedingDate)
		if err != nil {
			break
		}
		if !rate.ForeignToLocalRate.IsZero() {
			return rate, nil
		}
	}
	return DailyRate{}, fmt.Errorf(errFmt,
		"Could not find relevant exchange rate within the 7 preceding days")
}

func (cr *RateLoader) GetExactUsdCadRate(tradeDate date.Date) (DailyRate, error) {
	year := uint32(tradeDate.Year())
	yearRates, ok := cr.YearRates[year]
	if !ok {
		var err error
		yearRates, err = cr.fetchUsdCadRatesForDateYear(tradeDate)
		if err != nil {
			return DailyRate{}, err
		}
	}
	rate, ok := yearRates[tradeDate]
	if !ok {
		// if tradeDate >= today
		today := date.Today()
		if tradeDate == today || tradeDate.After(today) {
			// There is no rate available for today yet, so error out.
			// The user must manually provide a rate in this scenario.
			return DailyRate{}, fmt.Errorf(
				"No USD/CAD exchange rate is available for %s yet. Either explicitly add to "+
					"CSV file or modify the exchange rates cache file in ~/.acb/. "+
					"If today is a bank holiday, use rate for preceding business day.",
				tradeDate)
		}
		// There is no rate for this exact date, but it is for a date in the past,
		// so the caller can try a previous date for the relevant rate. (ie. we are
		// not in an error scenario yet).
		rate = DailyRate{}
	}
	return rate, nil
}

func (cr *RateLoader) GetEffectiveUsdCadRate(tradeDate date.Date) (DailyRate, error) {
	rate, err := cr.GetExactUsdCadRate(tradeDate)
	if err == nil {
		if rate.ForeignToLocalRate.IsZero() {
			rate, err = cr.findUsdCadPrecedingRelevantSpotRate(tradeDate, rate)
			if err == nil {
				return rate, nil
			}
		} else {
			return rate, nil
		}
	}
	return DailyRate{}, fmt.Errorf("Unable to retrieve exchange rate for %v: %s",
		tradeDate, err)
}
