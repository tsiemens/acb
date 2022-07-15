package test

import (
	"os"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/app"
	"github.com/tsiemens/acb/date"
	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

func validateSampleCsvFile(rq *require.Assertions, csvPath string) {
	fp, err := os.Open(csvPath)
	rq.Nil(err)
	defer fp.Close()
	csvReaders := []app.DescribedReader{app.DescribedReader{csvPath, fp}}

	errPrinter := &log.StderrErrorPrinter{}
	_, err = app.RunAcbAppToRenderModel(
		csvReaders, map[string]*ptf.PortfolioSecurityStatus{},
		false, false,
		app.LegacyOptions{},
		// fx.NewMemRatesCacheAccessor(),
		&fx.CsvRatesCache{ErrPrinter: errPrinter},
		errPrinter,
	)
	rq.Nil(err)
}

func TestSampleCsvFileValidity(t *testing.T) {
	rq := require.New(t)

	date.TodaysDateForTest = mkDateYD(2022, 1)
	wd, err := os.Getwd()
	rq.Nil(err)
	// If running the compiled test binary manually, it must be run from the test
	// directory. This is what happens when running 'go test ./test'
	rq.Regexp("test/?$", wd)

	validateSampleCsvFile(rq, "./test_combined.csv")
	validateSampleCsvFile(rq, "../www/html/sample_txs.csv")
}
