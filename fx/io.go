package fx

import (
	// "bytes"
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
	"strings"
	"time"

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

type RatesCache interface {
	WriteRates(year uint32, rates []DailyRate) error
	GetUsdCadRates(year uint32) ([]DailyRate, error)
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

func (cr *RateLoader) GetRemoteUsdCadRatesJson(year uint32, ratesCache RatesCache) ([]DailyRate, error) {
	cr.ErrPrinter.F("Fetching USD/CAD exchange rates for %d\n", year)
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
		date, err := time.Parse(csvTimeFormat, obs.Date)
		if err != nil {
			cr.ErrPrinter.Ln("Unable to parse date:", err)
			continue
		}

		var dRate DailyRate
		usdCadNoonVal, err := obs.UsdCadNoon.Val()
		if err != nil {
			cr.ErrPrinter.Ln("Failed to parse USDCAD Noon rate for", date, ":", obs.UsdCadNoon.ValStr)
			continue
		}

		if usdCadNoonVal != 0.0 {
			dRate = DailyRate{date, usdCadNoonVal}
		} else {
			usdCadVal, err := obs.UsdCad.Val()
			if err != nil {
				cr.ErrPrinter.Ln("Failed to parse USDCAD rate for", date, ":", obs.UsdCad.ValStr)
				continue
			}
			dRate = DailyRate{date, 1.0 / usdCadVal}
		}
		rates = append(rates, dRate)
	}

	err = ratesCache.WriteRates(year, rates)
	if err != nil {
		cr.ErrPrinter.Ln("Failed to update exchange rate cache:", err)
	}
	return rates, nil
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
		date, err := time.Parse(csvTimeFormat, record[0])
		if err != nil {
			c.ErrPrinter.Ln("Unable to parse date:", err)
			continue
		}
		rate, err := strconv.ParseFloat(record[1], 64)
		if err != nil {
			c.ErrPrinter.Ln("Unable to parse rate:", err)
			continue
		}

		dRate := DailyRate{date, rate}
		rates = append(rates, dRate)
	}

	return rates, nil
}

func (cr *RateLoader) GetUsdCadRatesForYear(
	year uint32, forceDownload bool, ratesCache RatesCache) ([]DailyRate, error) {

	if forceDownload {
		return cr.GetRemoteUsdCadRatesJson(year, ratesCache)
	}
	rates, err := ratesCache.GetUsdCadRates(year)
	if err != nil {
		cr.ErrPrinter.Ln("Could not load cached exchange rates:", err)
		return cr.GetRemoteUsdCadRatesJson(year, ratesCache)
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
	year, month, day := r.Date.Date()
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
	YearRates     map[uint32]map[time.Time]DailyRate
	ForceDownload bool
	Cache         RatesCache
	ErrPrinter    log.ErrorPrinter
}

func NewRateLoader(
	forceDownload bool, ratesCache RatesCache, errPrinter log.ErrorPrinter) *RateLoader {
	return &RateLoader{
		YearRates:     make(map[uint32]map[time.Time]DailyRate),
		ForceDownload: forceDownload,
		Cache:         ratesCache,
		ErrPrinter:    errPrinter,
	}
}

func tryGetSurroundingRates(t time.Time, yearRates map[time.Time]DailyRate) (beforeRate *DailyRate, afterRate *DailyRate) {
	beforeTime := t
	for i := 0; i < 7; i++ {
		beforeTime = beforeTime.AddDate(0, 0, -1)
		rate, ok := yearRates[beforeTime]
		if ok {
			beforeRate = &DailyRate{}
			*beforeRate = rate
			break
		}
	}
	afterTime := t
	for i := 0; i < 7; i++ {
		afterTime = afterTime.AddDate(0, 0, 1)
		rate, ok := yearRates[afterTime]
		if ok {
			afterRate = &DailyRate{}
			*afterRate = rate
			break
		}
	}
	// implicit return
	return
}

func getSurroundingRatesHelp(t time.Time, yearRates map[time.Time]DailyRate, prefix string) string {
	beforeRate, afterRate := tryGetSurroundingRates(t, yearRates)
	if beforeRate != nil || afterRate != nil {
		var builder strings.Builder
		builder.WriteString(prefix)
		builder.WriteString("If date is on a day where markets are closed, check if " +
			"date should be moved to another day.\nAlternatively, you may provide a " +
			"manual exchange rate from the appropriate surrounding day (NOTE these are only suggested rates, and do not currently include rates from different years. All saved FX rates can be found in ~/.acb/):")
		if beforeRate != nil {
			builder.WriteString("\n")
			builder.WriteString(util.DateStr(beforeRate.Date))
			builder.WriteString(": ")
			builder.WriteString(fmt.Sprintf("%f", beforeRate.ForeignToLocalRate))
		}
		if afterRate != nil {
			builder.WriteString("\n")
			builder.WriteString(util.DateStr(afterRate.Date))
			builder.WriteString(": ")
			builder.WriteString(fmt.Sprintf("%f", afterRate.ForeignToLocalRate))
		}
		return builder.String()
	}
	return ""
}

func (cr *RateLoader) GetUsdCadRate(t time.Time) (DailyRate, error) {
	yearRates, ok := cr.YearRates[uint32(t.Year())]
	if !ok {
		rates, err := cr.GetUsdCadRatesForYear(uint32(t.Year()), cr.ForceDownload, cr.Cache)
		if err != nil {
			return DailyRate{}, err
		}
		yearRates = make(map[time.Time]DailyRate)
		for _, rate := range rates {
			yearRates[rate.Date] = rate
		}
		cr.YearRates[uint32(t.Year())] = yearRates
	}
	rate, ok := yearRates[t]
	if !ok {
		return DailyRate{}, fmt.Errorf("Unable to retrieve exchange rate for %v%s", t,
			getSurroundingRatesHelp(t, yearRates, "\n"))
	}
	return rate, nil
}
