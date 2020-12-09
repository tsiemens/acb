package main

import (
	"fmt"
	"strings"
	"syscall/js"

	"github.com/tsiemens/acb/app"
	"github.com/tsiemens/acb/fx"
	ptf "github.com/tsiemens/acb/portfolio"
)

var globalRatesCache map[uint32][]fx.DailyRate = make(map[uint32][]fx.DailyRate)

func main() {
	fmt.Println("Go Web Assembly")
	js.Global().Set("golangDemo", golangDemoWrapper())
	js.Global().Set("runAcb", makeRunAcbWrapper())
	// Wait for calls
	<-make(chan bool)
}

type Something struct {
	A string
	B string
}

func golangDemo(input string) (map[string]interface{}, error) {
	// return fmt.Sprintf("'%s' formatted by golang", input), nil
	return map[string]interface{}{"A": "Its a A", "B": "Its a B"}, nil
}

func golangDemoWrapper() js.Func {
	wrapperFunc := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		if len(args) != 1 {
			return fmt.Errorf("Invalid no of arguments passed").Error()
		}
		inputStr := args[0].String()
		fmt.Printf("input %s\n", inputStr)
		out, err := golangDemo(inputStr)
		if err != nil {
			fmt.Printf("unable to run demo: %s\n", err)
			return err.Error()
		}
		return out
	})
	return wrapperFunc
}

type MemRatesCache struct{}

func (c *MemRatesCache) WriteRates(year uint32, rates []fx.DailyRate) error {
	globalRatesCache[year] = rates
	return nil
}

func (c *MemRatesCache) GetUsdCadRates(year uint32) ([]fx.DailyRate, error) {
	rates, ok := globalRatesCache[year]
	if !ok {
		return nil, fmt.Errorf("No cached USD/CAD rates for %d", year)
	}
	return rates, nil
}

// Map is the description mapped to the contents
func runAcb(csvDescs []string, csvContents []string) {
	fmt.Println("runAcb")
	csvReaders := make([]app.DescribedReader, 0, len(csvContents))
	for i, contents := range csvContents {
		desc := csvDescs[i]
		csvReaders = append(csvReaders, app.DescribedReader{desc, strings.NewReader(contents)})
	}

	allInitStatus := map[string]*ptf.PortfolioSecurityStatus{}
	forceDownload := false
	superficialLosses := true

	app.RunAcbApp(
		csvReaders, allInitStatus, forceDownload, superficialLosses, &MemRatesCache{})
}

func makeRetVal(ret interface{}, err error) interface{} {
	if err != nil {
		return js.ValueOf(map[string]interface{}{"ret": ret, "error": err.Error()})
	}
	return js.ValueOf(map[string]interface{}{"ret": ret})
}

func makeRunAcbWrapper() js.Func {
	wrapperFunc := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		if len(args) != 2 {
			return makeRetVal(
				nil, fmt.Errorf("Invalid number of arguments (%d). Expected 2", len(args)))
		}
		// These are expected to be array
		csvDescs := args[0]
		csvContents := args[1]
		if csvDescs.Type() != js.TypeObject || csvContents.Type() != js.TypeObject {
			return makeRetVal(
				nil, fmt.Errorf("Invalid argument of type (non-object)"))
		}

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

		runAcb(descs, contents)
		return makeRetVal(nil, nil)
	})
	return wrapperFunc
}
