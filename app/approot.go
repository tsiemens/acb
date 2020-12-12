package app

import (
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

/* Takes a list of security status strings, each formatted as:
 * SYM:nShares:totalAcb. Eg. GOOG:20:1000.00
 */
func ParseInitialStatus(
	initialSecurityStates []string) (map[string]*ptf.PortfolioSecurityStatus, error) {
	stati := make(map[string]*ptf.PortfolioSecurityStatus)
	for _, opt := range initialSecurityStates {
		parts := strings.Split(opt, ":")
		if len(parts) != 3 {
			return nil, fmt.Errorf("Invalid ACB format '%s'", opt)
		}
		symbol := parts[0]
		shares, err := strconv.ParseUint(parts[1], 10, 32)
		if err != nil {
			return nil, fmt.Errorf("Invalid shares format '%s'. %v", opt, err)
		}
		acb, err := strconv.ParseFloat(parts[2], 64)
		if err != nil {
			return nil, fmt.Errorf("Invalid ACB format '%s'. %v", opt, err)
		}

		if _, ok := stati[symbol]; ok {
			return nil, fmt.Errorf("Symbol %s specified multiple times", symbol)
		}
		stati[symbol] = &ptf.PortfolioSecurityStatus{
			Security: symbol, ShareBalance: uint32(shares), TotalAcb: acb}
	}
	return stati, nil
}

// Returns a map of Security to its RenderTable
/*
func CalculateAcbTables(
	initialAcbs map[string]*ptf.PortfolioSecurityStatus, forceDownload bool) (map[string]*RenderTable, error) {

	rateLoader := fx.NewRateLoader(ForceDownload)

}
*/

type DescribedReader struct {
	Desc   string
	Reader io.Reader
}

func RunAcbAppToModel(
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	applySuperficialLosses bool,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) (map[string]*ptf.RenderTable, error) {

	rateLoader := fx.NewRateLoader(forceDownload, ratesCache, errPrinter)

	allTxs := make([]*ptf.Tx, 0, 20)
	for _, csvReader := range csvFileReaders {
		txs, err := ptf.ParseTxCsv(csvReader.Reader, csvReader.Desc, rateLoader)
		if err != nil {
			return nil, err
		}
		for _, tx := range txs {
			allTxs = append(allTxs, tx)
		}
	}

	allTxs = ptf.SortTxs(allTxs)
	txsBySec := ptf.SplitTxsBySecurity(allTxs)

	models := make(map[string]*ptf.RenderTable)

	nSecs := len(txsBySec)
	i := 0
	for sec, secTxs := range txsBySec {
		secInitStatus, ok := allInitStatus[sec]
		if !ok {
			secInitStatus = nil
		}
		deltas, err := ptf.TxsToDeltaList(secTxs, secInitStatus, applySuperficialLosses)

		tableModel := ptf.RenderTxTableModel(deltas)
		if err != nil {
			tableModel.Errors = append(tableModel.Errors, err)
		}
		models[sec] = tableModel

		if i < (nSecs - 1) {
			fmt.Println("")
		}
		i++
	}
	return models, nil
}

// Returns an OK flag. Used to signal what exit code to use.
// All errors get printed to the errPrinter or to the writer (as appropriate).
func RunAcbAppToWriter(
	writer io.Writer,
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	applySuperficialLosses bool,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) bool {

	renderTables, err := RunAcbAppToModel(
		csvFileReaders, allInitStatus, forceDownload,
		applySuperficialLosses, ratesCache, errPrinter,
	)

	if err != nil {
		errPrinter.Ln("Error:", err)
		return false
	}

	nSecs := len(renderTables)
	i := 0
	for sec, renderTable := range renderTables {
		for _, err := range renderTable.Errors {
			fmt.Fprintf(writer, "[!] %v. Printing parsed information state:\n", err)
		}
		fmt.Fprintf(writer, "Transactions for %s\n", sec)
		ptf.PrintRenderTable(renderTable, writer)
		if i < (nSecs - 1) {
			fmt.Fprintln(writer, "")
		}
		i++
	}

	return true
}

// Returns an OK flag. Used to signal what exit code to use.
func RunAcbAppToConsole(
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	applySuperficialLosses bool,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) bool {

	return RunAcbAppToWriter(
		os.Stdout,
		csvFileReaders, allInitStatus, forceDownload,
		applySuperficialLosses, ratesCache, errPrinter,
	)
}
