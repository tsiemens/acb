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
	// Wait for calls
	<-make(chan bool)
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

// Map is the description mapped to the contents
func runAcb(csvDescs []string, csvContents []string) (string, error) {
	fmt.Println("runAcb")
	csvReaders := make([]app.DescribedReader, 0, len(csvContents))
	for i, contents := range csvContents {
		desc := csvDescs[i]
		csvReaders = append(csvReaders, app.DescribedReader{desc, strings.NewReader(contents)})
	}

	allInitStatus := map[string]*ptf.PortfolioSecurityStatus{}
	forceDownload := false
	superficialLosses := true

	errPrinter := &BufErrorPrinter{}

	var output strings.Builder

	app.RunAcbAppToWriter(
		&output,
		csvReaders, allInitStatus, forceDownload,
		superficialLosses, &GlobalMemRatesCacheAccessor{}, errPrinter,
	)

	outString := output.String()

	errString := errPrinter.Buf.String()
	if errString != "" {
		return outString, errors.New(errString)
	}
	return outString, nil
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

func makeRunAcbWrapper() js.Func {
	wrapperFunc := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		err := validateFuncArgs(args, js.TypeObject, js.TypeObject)
		if err != nil {
			return makeRetVal(nil, err)
		}

		// These are expected to be array
		csvDescs := args[0]
		csvContents := args[1]

		descs := make([]string, 0, csvContents.Length())
		contents := make([]string, 0, csvContents.Length())
		for i := 0; i < csvContents.Length(); i++ {
			c := csvContents.Index(i)
			if c.Type() != js.TypeString {
				return makeRetVal(
					nil, fmt.Errorf("Array item at index %d is not a string", i))
			}
			contents = append(contents, c.String())

			desc := ""
			if i < csvDescs.Length() {
				dVal := csvDescs.Index(i)
				if dVal.Type() != js.TypeString {
					return makeRetVal(
						nil, fmt.Errorf("Array item at index %d is not a string", i))
				}
				desc = dVal.String()
			}
			descs = append(descs, desc)
			fmt.Printf("%d: %s\n", i, desc)
		}

		promise := makeJsPromise(
			func(resolveFunc js.Value, rejectFunc js.Value) {
				go func() {
					out, err := runAcb(descs, contents)
					resolveFunc.Invoke(makeRetVal(out, err))
					// rejectFunc.Invoke("something error")
				}()
			})
		return makeRetVal(promise, nil)
	})
	return wrapperFunc
}
