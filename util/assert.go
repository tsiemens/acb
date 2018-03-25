package util

import (
	"fmt"
	"os"
	"runtime/debug"
)

// Can be set by tests if they want to catch asserts
var AssertsPanic bool = false

func Assert(cond bool, o ...interface{}) {
	if !cond {
		if AssertsPanic {
			panic(fmt.Sprint(o...))
		} else {
			debug.PrintStack()
			fmt.Fprint(os.Stderr, o...)
			os.Exit(1)
		}
	}
}

func Assertf(cond bool, fmtstr string, o ...interface{}) {
	if !cond {
		if AssertsPanic {
			panic(fmt.Sprintf(fmtstr, o...))
		} else {
			debug.PrintStack()
			fmt.Fprintf(os.Stderr, fmtstr, o...)
			os.Exit(1)
		}
	}
}
