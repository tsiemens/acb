package main

import (
	"fmt"
	"syscall/js"
)

func main() {
	fmt.Println("Go Web Assembly")
	js.Global().Set("golangDemo", golangDemoWrapper())
	// Wait for calls
	<-make(chan bool)
}

func golangDemo(input string) (string, error) {
	return fmt.Sprintf("'%s' formatted by golang", input), nil
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
