package test

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

func TestToCsvString(t *testing.T) {
	defer resetContext()
	rq := require.New(t)

	rateLoader := fx.NewRateLoader(
		false,
		fx.NewMemRatesCacheAccessor(),
		&log.StderrErrorPrinter{})

	verifyParsedTxs := func(txs []*ptf.Tx) {
		rq.Equal(len(txs), 2)

		header := "security,date,action,shares,amount/share,commission,currency," +
			"exchange rate,commission currency,commission exchange rate,memo\n"

		csvOut := ptf.ToCsvString(txs)
		rq.Equal(header+"FOO,2016-01-05,Sell,5,1.600000,0.000000,CAD,,CAD,,a memo\n"+
			"BAR,2016-01-06,Buy,7,1.700000,1.000000,USD,1.110000,USD,1.110000,a memo 2\n",
			csvOut)
	}

	// Modern input with trade and settlement dates.
	ctx.UseLegacyCsvHeaders = false
	csvReader := splitCsvRows([]uint32{2},
		"FOO,2016-01-03,2016-01-05,Sell,5,1.6,CAD,,0,a memo",
		"BAR,2016-01-03,2016-01-06,Buy,7,1.7,USD,1.11,1.0,a memo 2",
	)[0]
	txs, err := ptf.ParseTxCsv(csvReader.Reader, 0, "", rateLoader)
	rq.Nil(err)
	verifyParsedTxs(txs)

	// Legacy input, with just Date column
	ctx.UseLegacyCsvHeaders = true
	csvReader = splitCsvRows([]uint32{2},
		"FOO,2016-01-05,Sell,5,1.6,CAD,,0,a memo",
		"BAR,2016-01-06,Buy,7,1.7,USD,1.11,1.0,a memo 2",
	)[0]
	txs, err = ptf.ParseTxCsv(csvReader.Reader, 0, "", rateLoader)
	rq.Nil(err)
	verifyParsedTxs(txs)

	resetContext()
}

// Test the case where someone mistakenly adds the "date" column in combination
// with settlement date, which are for the same thing.
func TestDoubleSettlementDate(t *testing.T) {
	defer resetContext()
	rq := require.New(t)

	rateLoader := fx.NewRateLoader(
		false,
		fx.NewMemRatesCacheAccessor(),
		&log.StderrErrorPrinter{})

	ctx.CsvHeaders = "security,date,settlement date\n"
	csvReader := splitCsvRows([]uint32{2},
		"FOO,2016-01-03,2016-01-05",
		"BAR,2016-01-03,2016-01-06",
	)[0]
	_, err := ptf.ParseTxCsv(csvReader.Reader, 0, "", rateLoader)
	rq.Equal(err, fmt.Errorf("Error parsing  at line:col 1:2: Settlement Date provided twice (found both 'date' and 'settlement date' columns)"))
}
