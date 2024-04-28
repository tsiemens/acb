package test

import (
	"fmt"
	"strings"
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
		header := "security,trade date,settlement date,action,shares,amount/share,commission,currency," +
			"exchange rate,commission currency,commission exchange rate,superficial loss,affiliate,memo\n"

		txRows := "FOO,2016-01-03,2016-01-05,Sell,5,1.6,0,CAD,,CAD,,,Default,a memo\n" +
			"BAR,2016-01-03,2016-01-06,Buy,7,1.7,1,USD,1.11,USD,1.11,,Default,a memo 2\n" +
			"AA,2016-01-04,2016-01-07,Sell,1,1.7,1,USD,1.11,USD,1.11,-1.2,Default,M3\n" +
			"BB,2016-01-05,2016-01-08,Sell,2,1.7,1,USD,1.11,USD,1.11,-1.3!,Default (R),M4\n" +
			"CC,2016-01-08,2016-01-10,SfLA,2,1.3,0,CAD,,CAD,,,B,M5\n"

		rq.Equal(len(txs), strings.Count(txRows, "\n"))
		csvOut := ptf.ToCsvString(txs)

		crq := NewCustomRequire(t)
		crq.LinesEqual(header+txRows, csvOut)
	}

	// Modern input with trade and settlement dates.
	ctx.UseLegacyCsvHeaders = false
	// "security,trade date,settlement date,action,shares,amount/share,currency,exchange rate,commission,affiliate,memo,superficial loss\n"
	csvRows := []string{
		"FOO,     2016-01-03,2016-01-05,     Sell,  5,     1.6,         CAD,     ,             0,         , a memo,",
		"BAR,     2016-01-03,2016-01-06,     Buy,   7,     1.7,         USD,     1.11,         1.0,       default, a memo 2,",
		"AA,      2016-01-04,2016-01-07,     Sell,  1,     1.7,         USD,     1.11,         1.0,       Default, M3,  -1.2",
		"BB,      2016-01-05,2016-01-08,     Sell,  2,     1.7,         USD,     1.11,         1.0,       (R), M4,  -1.3!",
		"CC,      2016-01-08,2016-01-10,     SfLA,  2,     1.3,         CAD,     ,             ,          B, M5,",
	}
	csvReader := splitCsvRows([]uint32{uint32(len(csvRows))}, csvRows...)[0]
	txs, err := ptf.ParseTxCsv(csvReader.Reader, 0, "", rateLoader)
	rq.Nil(err)
	verifyParsedTxs(txs)

	// Legacy input, with just Date column (no longer allowed)
	ctx.UseLegacyCsvHeaders = true
	csvReader = splitCsvRows([]uint32{2},
		"FOO,2016-01-05,Sell,5,1.6,CAD,,0,a memo,",
		"BAR,2016-01-06,Buy,7,1.7,USD,1.11,1.0,a memo 2,",
	)[0]
	txs, err = ptf.ParseTxCsv(csvReader.Reader, 0, "", rateLoader)
	rq.Empty(txs)
	rq.NotNil(err)
	rq.Contains(err.Error(), "Transaction has no trade date")

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
