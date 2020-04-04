package log

import (
	"fmt"
	"io"
)

var VerboseEnabled = false

func Fverbosef(w io.Writer, format string, v ...interface{}) {
	if VerboseEnabled {
		fmt.Fprintf(w, format, v...)
	}
}
