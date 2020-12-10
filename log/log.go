package log

import (
	"fmt"
	"io"
	"os"
)

var VerboseEnabled = false

func Fverbosef(w io.Writer, format string, v ...interface{}) {
	if VerboseEnabled {
		fmt.Fprintf(w, format, v...)
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
