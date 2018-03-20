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
)

const (
	cadUsdJsonUrl = "https://www.bankofcanada.ca/valet/observations/FXUSDCAD/json?start_date=2017-01-01&end_date=2017-02-01"

	// "?start_date=2016-01-01&end_date=2016-12-31"
	cadUsdNoonUrl = "https://www.bankofcanada.ca/valet/observations/IEXE0101/csv?start_date=2016-01-01&end_date=2016-12-31"
	// cadUsdNoonUrl = "https://www.bankofcanada.ca/valet/observations/IEXE0101/csv?start_date=%s&end_date=%s"
	lineBufSize     = 100
	csvTimeFormat   = "2006-01-02"
	csvPrintTimeFmt = "%d-%02d-%02d"
)

type ValetJsonFx struct {
	Val float64 `json:"v"`
}

type ValetJsonObs struct {
	Date   string      `json:"d"`
	UsdCad ValetJsonFx `json:"FXUSDCAD"`
}

type ValetJsonRoot struct {
	Observations []ValetJsonObs `json:"observations"`
}

func GetRemoteCadUsdRatesJson() ([]DailyRate, error) {
	resp, err := http.Get(cadUsdJsonUrl)
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
		dRate := DailyRate{date, obs.UsdCad.Val}
		rates = append(rates, dRate)
	}

	err = WriteRatesToCsv(rates)
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

func GetRemoteCadUsdRates() ([]DailyRate, error) {
	resp, err := http.Get(cadUsdNoonUrl)
	if err != nil {
		return nil, fmt.Errorf("Error getting CAD USD rates: %v", err)
	} else if resp.StatusCode != 200 {
		return nil, fmt.Errorf("Error status: %s", resp.Status)
	}

	var line string
	var lineB [lineBufSize]byte
	seekBuf := make([]byte, 1)

	// Seek to start of CSV
	foundStart := false
	lineIdx := 0
	for !foundStart {
		n, err := resp.Body.Read(seekBuf)
		if err != nil {
			return nil, fmt.Errorf("Error reading body: %v", err)
		} else if n == 0 {
			return nil, fmt.Errorf("Cound not find data part in csv")
		}
		if seekBuf[0] == '\n' {
			line = string(lineB[:lineIdx])
			lineIdx = 0
			if strings.Contains(line, "date,IEXE0101") {
				foundStart = true
			}
		} else {
			lineB[lineIdx] = seekBuf[0]
			lineIdx += 1
		}
	}

	rates, err := getRatesFromCsv(resp.Body)
	if err != nil {
		return nil, err
	}
	err = WriteRatesToCsv(rates)
	if err != nil {
		fmt.Fprintln(os.Stderr, "Failed to update exchange rate cache:", err)
	}
	return rates, nil
}

func GetCachedCadUsdRates() ([]DailyRate, error) {
	file, err := ratesCsvFile(false)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	return getRatesFromCsv(file)
}

func GetCadUsdRates(forceDownload bool) ([]DailyRate, error) {
	if forceDownload {
		return GetRemoteCadUsdRatesJson()
	}
	rates, err := GetCachedCadUsdRates()
	if err != nil {
		fmt.Println("Error getting cached exchange rates:", err, "\nTrying to get from remote.")
		return GetRemoteCadUsdRatesJson()
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

func ratesCsvFile(write bool) (*os.File, error) {
	preFname := "rates.csv"
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

func WriteRatesToCsv(rates []DailyRate) (err error) {
	err = nil
	file, err := ratesCsvFile(true)
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
