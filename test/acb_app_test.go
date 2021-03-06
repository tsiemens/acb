package test

import (
	"fmt"
	"os"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/app"
	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

const header = "security,date,action,shares,amount/share,currency,exchange rate,commission,memo\n"

func makeCsvReader(desc string, lines ...string) app.DescribedReader {
	contents := strings.Join(lines, "\n")
	return app.DescribedReader{desc, strings.NewReader(header + contents)}
}

func render(tableModel *ptf.RenderTable) {
	if os.Getenv("VERBOSE") != "" {
		ptf.PrintRenderTable(tableModel, os.Stdout)
	}
}

func splitCsvRows(fileLens []uint32, rows ...string) []app.DescribedReader {
	rowsRead := 0
	csvReaders := make([]app.DescribedReader, 0, len(fileLens))
	for i, fileLen := range fileLens {
		csvReaders = append(csvReaders, makeCsvReader(
			fmt.Sprintf("foo%d.csv", i),
			rows[rowsRead:rowsRead+int(fileLen)]...,
		))
		rowsRead += int(fileLen)
	}
	return csvReaders
}

func getTotalCapGain(tableModel *ptf.RenderTable) string {
	return tableModel.Footer[8]
}

func getAndCheckFooTable(rq *require.Assertions, rts map[string]*ptf.RenderTable) *ptf.RenderTable {
	rq.NotNil(rts)
	rq.Equal(1, len(rts))
	renderTable := rts["FOO"]
	rq.NotNil(renderTable)
	render(renderTable)
	return renderTable
}

func TestSameDayBuySells(t *testing.T) {
	rq := require.New(t)

	for _, splits := range [][]uint32{[]uint32{3}, []uint32{1, 2}} {
		csvReaders := splitCsvRows(splits,
			"FOO,2016-01-05,Buy,20,1.5,CAD,,0,",
			"FOO,2016-01-05,Sell,5,1.6,CAD,,0,",
			"FOO,2016-01-05,Buy,5,1.7,CAD,,0,",
		)

		renderTables, err := app.RunAcbAppToModel(
			csvReaders, map[string]*ptf.PortfolioSecurityStatus{},
			false, false,
			app.LegacyOptions{},
			fx.NewMemRatesCacheAccessor(),
			&log.StderrErrorPrinter{},
		)

		AssertNil(t, err)
		renderTable := getAndCheckFooTable(rq, renderTables)
		rq.Equal(3, len(renderTable.Rows))
		rq.ElementsMatch([]error{}, renderTable.Errors)
		rq.Equal("$0.50", getTotalCapGain(renderTable))

		// Try with legacy buys before sell
		csvReaders = splitCsvRows(splits,
			"FOO,2016-01-05,Buy,20,1.5,CAD,,0,",
			"FOO,2016-01-05,Sell,5,1.6,CAD,,0,",
			"FOO,2016-01-05,Buy,5,1.7,CAD,,0,",
		)

		renderTables, err = app.RunAcbAppToModel(
			csvReaders, map[string]*ptf.PortfolioSecurityStatus{},
			false, false,
			app.LegacyOptions{SortBuysBeforeSells: true},
			fx.NewMemRatesCacheAccessor(),
			&log.StderrErrorPrinter{},
		)

		AssertNil(t, err)
		renderTable = getAndCheckFooTable(rq, renderTables)
		rq.Equal(3, len(renderTable.Rows))
		rq.ElementsMatch([]error{}, renderTable.Errors)
		rq.Equal("$0.30", getTotalCapGain(renderTable))
	}
}

func TestNegativeStocks(t *testing.T) {
	rq := require.New(t)

	csvReaders := splitCsvRows([]uint32{1},
		"FOO,2016-01-05,Sell,5,1.6,CAD,,0,",
	)

	renderTables, err := app.RunAcbAppToModel(
		csvReaders, map[string]*ptf.PortfolioSecurityStatus{},
		false, false,
		app.LegacyOptions{},
		fx.NewMemRatesCacheAccessor(),
		&log.StderrErrorPrinter{},
	)

	AssertNil(t, err)
	renderTable := getAndCheckFooTable(rq, renderTables)
	rq.Equal(0, len(renderTable.Rows))
	rq.Contains(renderTable.Errors[0].Error(), "is more than the current holdings")
	rq.Equal("$0.00", getTotalCapGain(renderTable))

}
