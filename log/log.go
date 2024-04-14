package log

import (
	"fmt"
	"io"
	"os"
	"strings"
)

// This won't be as verbose as tracing, which is likely for testing only.
var VerboseEnabled = false

func Fverbosef(w io.Writer, format string, v ...interface{}) {
	if VerboseEnabled {
		fmt.Fprintf(w, format, v...)
	}
}

var tracingLoaded = false

// Tags enabled. Value ignored
var TraceSetting = map[string]bool{}

// Supply the TRACE environment variable with a comma-separated list of
// trace tags to enable.
func LoadTraceSetting() {
	tracingLoaded = true
	traceVar := os.Getenv("TRACE")
	if traceVar != "" {
		tags := strings.Split(traceVar, ",")
		for _, tag := range tags {
			TraceSetting[tag] = true
		}
	}
}

func MaybeLoadTraceSetting() {
	if !tracingLoaded {
		LoadTraceSetting()
	}
}

func Tracef(tag string, format string, v ...interface{}) {
	MaybeLoadTraceSetting()
	if _, ok := TraceSetting[tag]; ok {
		fmt.Fprintf(os.Stderr, "TR "+tag+" "+format+"\n", v...)
	}
}

type ErrorPrinter interface {
	Ln(v ...interface{})
	F(format string, v ...interface{})
}

// The default ErrorPrinter
type StderrErrorPrinter struct{}

func (p *StderrErrorPrinter) Ln(v ...interface{}) {
	fmt.Fprintln(os.Stderr, v...)
}

func (p *StderrErrorPrinter) F(format string, v ...interface{}) {
	fmt.Fprintf(os.Stderr, format, v...)
}
