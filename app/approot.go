package app

import (
	"fmt"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"

	"github.com/shopspring/decimal"

	"github.com/tsiemens/acb/date"
	decimal_opt "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
	ptf "github.com/tsiemens/acb/portfolio"
)

// Version is of the format 0.YY.MM[.i], or 0.year.month.optional_minor_increment
// This is similar to Ubuntu's versioning scheme, and allows for a more immediate
// reference for when the last time the app was updated.
// Major version is kept at 0, since the app is perpetually in 'beta' due to there
// not being a tax-lawer on staff to verify anything.
var AcbVersion = "0.23.04"

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
		shares, err := strconv.ParseFloat(parts[1], 10)
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
			Security: symbol, ShareBalance: decimal.NewFromFloat(shares), TotalAcb: decimal_opt.NewFromFloat(acb)}
	}
	return stati, nil
}

type DescribedReader struct {
	Desc   string
	Reader io.Reader
}

type LegacyOptions struct {
	// None currently
}

func NewLegacyOptions() LegacyOptions {
	return LegacyOptions{}
}

type Options struct {
	ForceDownload           bool
	RenderFullDollarValues  bool
	SummaryModeLatestDate   date.Date
	SplitAnnualSummaryGains bool
}

func (o *Options) SummaryMode() bool {
	return o.SummaryModeLatestDate != date.Date{}
}

func NewOptions() Options {
	return Options{
		ForceDownload:           false,
		RenderFullDollarValues:  false,
		SummaryModeLatestDate:   date.Date{},
		SplitAnnualSummaryGains: false,
	}
}

type SecurityDeltas struct {
	Deltas []*ptf.TxDelta
	Errors []error
}

func RunAcbAppToDeltaModels(
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	legacyOptions LegacyOptions,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) (map[string]*SecurityDeltas, error) {

	rateLoader := fx.NewRateLoader(forceDownload, ratesCache, errPrinter)

	allTxs := make([]*ptf.Tx, 0, 20)
	var globalReadIndex uint32 = 0
	for _, csvReader := range csvFileReaders {
		txs, err := ptf.ParseTxCsv(csvReader.Reader, globalReadIndex, csvReader.Desc, rateLoader)
		if err != nil {
			return nil, err
		}
		globalReadIndex += uint32(len(txs))
		for _, tx := range txs {
			allTxs = append(allTxs, tx)
		}
	}

	allTxs = ptf.SortTxs(allTxs)
	txsBySec := ptf.SplitTxsBySecurity(allTxs)

	portfolioLegacyOptions := ptf.LegacyOptions{}
	secModels := make(map[string]*SecurityDeltas)

	for sec, secTxs := range txsBySec {
		secInitStatus, ok := allInitStatus[sec]
		if !ok {
			secInitStatus = nil
		}
		deltas, err := ptf.TxsToDeltaList(secTxs, secInitStatus, portfolioLegacyOptions)
		deltasModel := &SecurityDeltas{deltas, []error{}}
		if err != nil {
			deltasModel.Errors = append(deltasModel.Errors, err)
		}
		secModels[sec] = deltasModel
	}
	return secModels, nil
}

type AllCumulativeCapitalGains struct {
	SecurityGains  map[string]*ptf.CumulativeCapitalGains
	AggregateGains *ptf.CumulativeCapitalGains
}

func getCumulativeCapitalGains(deltasBySec map[string]*SecurityDeltas) *AllCumulativeCapitalGains {
	securityGains := make(map[string]*ptf.CumulativeCapitalGains)
	for sec, deltas := range deltasBySec {
		securityGains[sec] = ptf.CalcSecurityCumulativeCapitalGains(deltas.Deltas)
	}
	aggregateGains := ptf.CalcCumulativeCapitalGains(securityGains)
	return &AllCumulativeCapitalGains{securityGains, aggregateGains}
}

type AppRenderResult struct {
	SecurityTables      map[string]*ptf.RenderTable
	AggregateGainsTable *ptf.RenderTable
}

func RunAcbAppToRenderModel(
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	renderFullDollarValues bool,
	legacyOptions LegacyOptions,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) (*AppRenderResult, error) {

	deltasBySec, err := RunAcbAppToDeltaModels(
		csvFileReaders, allInitStatus, forceDownload, legacyOptions, ratesCache,
		errPrinter)
	if err != nil {
		return nil, err
	}

	gains := getCumulativeCapitalGains(deltasBySec)

	secModels := make(map[string]*ptf.RenderTable)
	for sec, deltas := range deltasBySec {
		tableModel := ptf.RenderTxTableModel(
			deltas.Deltas, gains.SecurityGains[sec], renderFullDollarValues)
		tableModel.Errors = deltas.Errors
		secModels[sec] = tableModel
	}

	cumulativeGainsTable := ptf.RenderAggregateCapitalGains(
		gains.AggregateGains, renderFullDollarValues)

	return &AppRenderResult{secModels, cumulativeGainsTable}, nil
}

func RunAcbAppSummaryToModel(
	latestDate date.Date,
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	options Options,
	legacyOptions LegacyOptions,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) (*ptf.CollectedSummaryData, error) {

	secDeltasBySec, err := RunAcbAppToDeltaModels(
		csvFileReaders, allInitStatus, forceDownload, legacyOptions, ratesCache,
		errPrinter)
	if err != nil {
		return nil, err
	}

	deltasBySec := map[string][]*ptf.TxDelta{}
	errors := map[string][]error{}
	for sec, deltas := range secDeltasBySec {
		if deltas.Errors != nil && len(deltas.Errors) > 0 {
			errors[sec] = deltas.Errors
		}

		deltasBySec[sec] = deltas.Deltas
	}
	if len(errors) > 0 {
		return &ptf.CollectedSummaryData{Txs: nil, Warnings: nil, Errors: errors}, nil
	}

	return ptf.MakeAggregateSummaryTxs(latestDate, deltasBySec, options.SplitAnnualSummaryGains), nil
}

func WriteRenderResult(renderRes *AppRenderResult, writer io.Writer) {
	secRenderTables := renderRes.SecurityTables
	nSecs := len(secRenderTables)

	secs := make([]string, 0, len(secRenderTables))
	for k := range secRenderTables {
		secs = append(secs, k)
	}
	sort.Strings(secs)

	var secsWithErrors []string

	i := 0
	for _, sec := range secs {
		renderTable := secRenderTables[sec]
		ptf.PrintRenderTable(fmt.Sprintf("Transactions for %s", sec), renderTable, writer)
		if i < (nSecs - 1) {
			fmt.Fprintln(writer, "")
		}
		if len(renderTable.Errors) > 0 {
			secsWithErrors = append(secsWithErrors, sec)
		}
		i++
	}

	fmt.Fprintln(writer, "")
	ptf.PrintRenderTable("Aggregate Gains", renderRes.AggregateGainsTable, writer)

	if len(secsWithErrors) > 0 {
		fmt.Println("\n[!] There are errors for the following securities:", strings.Join(secsWithErrors, ", "))
	}
}

// Returns an OK flag. Used to signal what exit code to use.
// All errors get printed to the errPrinter or to the writer (as appropriate).
func RunAcbAppToWriter(
	writer io.Writer,
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	renderFullDollarValues bool,
	legacyOptions LegacyOptions,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) (bool, *AppRenderResult) {

	renderRes, err := RunAcbAppToRenderModel(
		csvFileReaders, allInitStatus, forceDownload, renderFullDollarValues,
		legacyOptions, ratesCache, errPrinter,
	)

	if err != nil {
		errPrinter.Ln("Error:", err)
		return false, nil
	}

	WriteRenderResult(renderRes, writer)
	return true, renderRes
}

func WriteSummaryData(summData *ptf.CollectedSummaryData, errPrinter log.ErrorPrinter) {
	if summData.Errors != nil && len(summData.Errors) > 0 {
		for sec, errs := range summData.Errors {
			errPrinter.F("Error(s) in %s:\n", sec)
			for _, err := range errs {
				errPrinter.F(" %s", err)
			}
		}
		return
	}

	if summData.Warnings != nil && len(summData.Warnings) > 0 {
		errPrinter.Ln("Warnings:")
		for warn, secs := range summData.Warnings {
			errPrinter.F(" %s. Encountered for %s\n", warn, strings.Join(secs, ","))
		}
		errPrinter.F("\n")
	}

	if summData.Txs != nil && len(summData.Txs) > 0 {
		fmt.Printf("%s", ptf.ToCsvString(summData.Txs))
	}
}

// Returns an OK flag. Used to signal what exit code to use.
func RunAcbAppSummaryToConsole(
	latestDate date.Date,
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	forceDownload bool,
	options Options,
	legacyOptions LegacyOptions,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) bool {

	summData, err := RunAcbAppSummaryToModel(
		latestDate, csvFileReaders, allInitStatus, forceDownload,
		options, legacyOptions, ratesCache, errPrinter)

	if err != nil {
		errPrinter.Ln("Error:", err)
		return false
	}

	WriteSummaryData(summData, errPrinter)
	return len(summData.Errors) == 0
}

// Returns an OK flag. Used to signal what exit code to use.
func RunAcbAppToConsole(
	csvFileReaders []DescribedReader,
	allInitStatus map[string]*ptf.PortfolioSecurityStatus,
	options Options,
	legacyOptions LegacyOptions,
	ratesCache fx.RatesCache,
	errPrinter log.ErrorPrinter) bool {

	ok := true
	if options.SummaryMode() {
		ok = RunAcbAppSummaryToConsole(
			options.SummaryModeLatestDate, csvFileReaders, allInitStatus,
			options.ForceDownload,
			options, legacyOptions, ratesCache, errPrinter,
		)
	} else {
		ok, _ = RunAcbAppToWriter(
			os.Stdout,
			csvFileReaders, allInitStatus, options.ForceDownload, options.RenderFullDollarValues,
			legacyOptions, ratesCache, errPrinter,
		)
	}
	return ok
}
