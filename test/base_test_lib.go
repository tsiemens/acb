package test

import (
	"testing"

	"github.com/stretchr/testify/require"
)

// regex can be pattern string or Regexp
func RqPanicsWithRegexp(t *testing.T, regex interface{}, fn func()) {
	defer func() {
		if r := recover(); r != nil {
			require.Regexp(t, regex, r)
		} else {
			require.FailNow(t, "Function did not panic")
		}
	}()
	fn()
}
