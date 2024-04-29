package main

import (
	"errors"
	"fmt"
	"os"
	"strings"
	"syscall/js"

	"github.com/tsiemens/acb/app"
	"github.com/tsiemens/acb/fx"
	ptf "github.com/tsiemens/acb/portfolio"
)

var globalRatesCache map[uint32][]fx.DailyRate = make(map[uint32][]fx.DailyRate)

func main() {
	fmt.Println("Go Web Assembly started")
	js.Global().Set("runAcb", makeRunAcbWrapper())
	js.Global().Set("getAcbVersion", makeGetVersionWrapper())
	// Wait for calls
	<-make(chan bool)
}

func makeGetVersionWrapper() js.Func {
	wrapperFunc := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		return app.AcbVersion
	})
	return wrapperFunc
}

// The default ErrorPrinter
type BufErrorPrinter struct {
	Buf strings.Builder
}

func (p *BufErrorPrinter) Ln(v ...interface{}) {
	fmt.Fprintln(&p.Buf, v...)
	fmt.Fprintln(os.Stderr, v...)
}

func (p *BufErrorPrinter) F(format string, v ...interface{}) {
	fmt.Fprintf(&p.Buf, format, v...)
	fmt.Fprintf(os.Stderr, format, v...)
}

type GlobalMemRatesCacheAccessor struct{}

func (c *GlobalMemRatesCacheAccessor) WriteRates(year uint32, rates []fx.DailyRate) error {
	globalRatesCache[year] = rates
	return nil
}

func (c *GlobalMemRatesCacheAccessor) GetUsdCadRates(year uint32) ([]fx.DailyRate, error) {
	rates, ok := globalRatesCache[year]
	if !ok {
		return nil, nil
	}
	return rates, nil
}

func stringArrayArrayToIntfArray(arr [][]string) []interface{} {
	outArr := make([]interface{}, 0, len(arr))
	for _, a := range arr {
		outArr = append(outArr, stringArrayToIntfArray(a))
	}
	return outArr
}

func stringArrayToIntfArray(arr []string) []interface{} {
	outArr := make([]interface{}, 0, len(arr))
	for _, s := range arr {
		outArr = append(outArr, s)
	}
	return outArr
}

func errorArrayToIntfArray(arr []error) []interface{} {
	outArr := make([]interface{}, 0, len(arr))
	for _, e := range arr {
		outArr = append(outArr, e.Error())
	}
	return outArr
}

func renderTableToJsConvertible(renderTable *ptf.RenderTable) map[string]interface{} {
	return map[string]interface{}{
		"header": stringArrayToIntfArray(renderTable.Header),
		"rows":   stringArrayArrayToIntfArray(renderTable.Rows),
		"footer": stringArrayToIntfArray(renderTable.Footer),
		"notes":  stringArrayToIntfArray(renderTable.Notes),
		"errors": errorArrayToIntfArray(renderTable.Errors),
	}
}

func renderTableToJsObject(renderTable *ptf.RenderTable) js.Value {
	if renderTable == nil {
		return js.ValueOf(nil)
	}
	return js.ValueOf(renderTableToJsConvertible(renderTable))
}

func renderTablesToJsObject(renderTables map[string]*ptf.RenderTable) js.Value {
	if renderTables == nil {
		return js.ValueOf(nil)
	}

	tableObjMap := map[string]interface{}{}
	for symbol, renderTable := range renderTables {
		tableObjMap[symbol] = renderTableToJsConvertible(renderTable)
	}

	return js.ValueOf(tableObjMap)
}

/* csvDescs: descriptions of each csv. usually just the name.
 * csvContents: The read contents of each csv file. Indexes must match csvDescs
 *	initialSymbolStates: list of symbol states formatted as  SYM:nShares:totalAcb.
 *                      Eg. GOOG:20:1000.00
 *
 * Returns a js object representation of a map[string]ptf.RenderTable
 */
func runAcb(
	csvDescs []string, csvContents []string,
	initialSymbolStates []string,
	renderFullValues bool,
	// Legacy options (none currently)
) (js.Value, error) {

	fmt.Println("runAcb")
	csvReaders := make([]app.DescribedReader, 0, len(csvContents))
	for i, contents := range csvContents {
		desc := csvDescs[i]
		csvReaders = append(csvReaders, app.DescribedReader{desc, strings.NewReader(contents)})
	}

	forceDownload := false

	allInitStatus, err := app.ParseInitialStatus(initialSymbolStates)
	if err != nil {
		return js.ValueOf(nil), err
	}

	errPrinter := &BufErrorPrinter{}

	var output strings.Builder

	legacyOptions := app.NewLegacyOptions()

	ok, renderRes := app.RunAcbAppToWriter(
		&output,
		csvReaders, allInitStatus, forceDownload, renderFullValues,
		legacyOptions, &fx.MemRatesCacheAccessor{RatesByYear: globalRatesCache},
		errPrinter,
	)

	outString := output.String()

	var secTables js.Value
	var aggTable js.Value
	if ok && renderRes != nil {
		secTables = renderTablesToJsObject(renderRes.SecurityTables)
		aggTable = renderTableToJsObject(renderRes.AggregateGainsTable)
	} else {
		secTables = js.ValueOf(nil)
		aggTable = js.ValueOf(nil)
	}

	outObj := js.ValueOf(map[string]interface{}{
		"textOutput": outString,
		"modelOutput": map[string]interface{}{
			"securityTables":      secTables,
			"aggregateGainsTable": aggTable,
		},
	})

	errString := errPrinter.Buf.String()
	if errString != "" {
		return outObj, errors.New(errString)
	}
	return outObj, nil
}

func makeRetVal(ret interface{}, err error) interface{} {
	if err != nil {
		return js.ValueOf(map[string]interface{}{"result": ret, "error": err.Error()})
	}
	return js.ValueOf(map[string]interface{}{"result": ret})
}

func makeJsPromise(
	promiseFunc func(resolveFunc js.Value, rejectFunc js.Value)) interface{} {

	handler := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		err := validateFuncArgs(args, js.TypeFunction, js.TypeFunction)
		if err != nil {
			fmt.Println("Error in promise handler: ", err)
			return nil
		}
		promiseFunc(args[0], args[1])
		return nil
	})

	promiseCtor := js.Global().Get("Promise")
	return promiseCtor.New(handler)
}

func makeErrorPromise(err error) interface{} {
	return makeJsPromise(
		func(resolveFunc js.Value, rejectFunc js.Value) {
			go func() {
				resolveFunc.Invoke(makeRetVal(nil, err))
				// rejectFunc.Invoke("something error")
			}()
		})
}

func validateFuncArgs(args []js.Value, types ...js.Type) error {
	if len(args) != len(types) {
		return fmt.Errorf("Invalid number of arguments (%d). Expected %d",
			len(args), len(types))
	}
	for i, typ := range types {
		if typ != args[i].Type() {
			return fmt.Errorf("Invalid type for argument %d. Got %s but expected %s.",
				i, args[i].Type().String(), typ.String())
		}
	}
	return nil
}

func jsArrayToStringArray(jsArr js.Value) ([]string, error) {
	arr := make([]string, 0, jsArr.Length())
	for i := 0; i < jsArr.Length(); i++ {
		v := jsArr.Index(i)
		if v.Type() != js.TypeString {
			return nil, fmt.Errorf("Array item at index %d is not a string", i)
		}
		arr = append(arr, v.String())
	}
	return arr, nil
}

func makeRunAcbWrapper() js.Func {
	wrapperFunc := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		err := validateFuncArgs(
			args, js.TypeObject, js.TypeObject, js.TypeObject, js.TypeBoolean)
		if err != nil {
			return makeErrorPromise(err)
		}

		popArgIdx := 0
		popArg := func() js.Value {
			i := popArgIdx
			popArgIdx++
			return args[i]
		}

		descs, err := jsArrayToStringArray(popArg())
		if err != nil {
			return makeErrorPromise(err)
		}
		contents, err := jsArrayToStringArray(popArg())
		if err != nil {
			return makeErrorPromise(err)
		}
		// We need a desc for each csv. Default them to empty string.
		for i := range contents {
			var desc string = ""
			if i < len(descs) {
				desc = descs[i]
			} else {
				descs = append(descs, "")
			}
			fmt.Printf("%d: %s\n", i, desc)
		}

		initialSymbolStates, err := jsArrayToStringArray(popArg())
		if err != nil {
			return makeErrorPromise(err)
		}

		renderFullValues := popArg().Bool()

		promise := makeJsPromise(
			func(resolveFunc js.Value, rejectFunc js.Value) {
				go func() {
					out, err := runAcb(
						descs, contents, initialSymbolStates, renderFullValues)
					resolveFunc.Invoke(makeRetVal(out, err))
					// rejectFunc.Invoke("something error")
				}()
			})
		return promise
	})
	return wrapperFunc
}
