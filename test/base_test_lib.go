package test

import (
	"math"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

var NaN float64 = math.NaN()

func IsAlmostEqual(a float64, b float64) bool {
	if math.IsNaN(a) && math.IsNaN(b) {
		return true
	}
	diff := a - b
	return diff < 0.0000001 && diff > -0.0000001
}

func SoftAlmostEqual(t *testing.T, exp float64, actual float64) bool {
	if IsAlmostEqual(exp, actual) {
		return true
	}
	// This should always fail
	return assert.Equal(t, exp, actual)
}

func AlmostEqual(t *testing.T, exp float64, actual float64) {
	if !SoftAlmostEqual(t, exp, actual) {
		t.FailNow()
	}
}

// Equal will fail with NaN == NaN, so we need some special help to make
// the failure pretty.
func RqNaN(t *testing.T, actual float64) {
	if !math.IsNaN(actual) {
		// This always fails, but will give some nice ouput
		require.Equal(t, NaN, actual)
	}
}

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
