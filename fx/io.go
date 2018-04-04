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
	"time"
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
	Val float64 `json:"v"`
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

func GetRemoteUsdCadRatesJson(year uint32) ([]DailyRate, error) {
	fmt.Fprintf(os.Stderr, "Fetching USD/CAD exchange rates for %d\n", year)
	resp, err := http.Get(getJsonUrl(year))
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
			fmt.Fprintln(os.Stderr, "Unable to parse date:", err)
			continue
		}

		var dRate DailyRate
		if obs.UsdCadNoon.Val != 0.0 {
			dRate = DailyRate{date, obs.UsdCadNoon.Val}
		} else {
			dRate = DailyRate{date, 1.0 / obs.UsdCad.Val}
		}
		rates = append(rates, dRate)
	}

	err = WriteRatesToCsv(year, rates)
	if err != nil {
		fmt.Fprintln(os.Stderr, "Failed to update exchange rate cache:", err)
	}
	return rates, nil
}

func getRatesFromCsv(r io.Reader) ([]DailyRate, error) {
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
			fmt.Fprintln(os.Stderr, "Unable to parse date:", err)
			continue
		}
		rate, err := strconv.ParseFloat(record[1], 64)
		if err != nil {
			fmt.Fprintln(os.Stderr, "Unable to parse rate:", err)
			continue
		}

		dRate := DailyRate{date, rate}
		rates = append(rates, dRate)
	}

	return rates, nil
}

func GetCachedUsdCadRates(year uint32) ([]DailyRate, error) {
	file, err := ratesCsvFile(year, false)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	return getRatesFromCsv(file)
}

func GetUsdCadRatesForYear(year uint32, forceDownload bool) ([]DailyRate, error) {
	if forceDownload {
		return GetRemoteUsdCadRatesJson(year)
	}
	rates, err := GetCachedUsdCadRates(year)
	if err != nil {
		fmt.Println("Could not load cached exchange rates:", err)
		return GetRemoteUsdCadRatesJson(year)
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
}

func NewRateLoader(forceDownload bool) *RateLoader {
	return &RateLoader{
		YearRates:     make(map[uint32]map[time.Time]DailyRate),
		ForceDownload: forceDownload,
	}
}

func (cr *RateLoader) GetUsdCadRate(t time.Time) (DailyRate, error) {
	yearRates, ok := cr.YearRates[uint32(t.Year())]
	if !ok {
		rates, err := GetUsdCadRatesForYear(uint32(t.Year()), cr.ForceDownload)
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
		return DailyRate{}, fmt.Errorf("Unable to retrieve exchange rate for %v", t)
	}
	return rate, nil
}
