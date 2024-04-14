package test

import (
	"strings"
	"testing"

	"github.com/google/go-cmp/cmp"
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

// Use this class instead of require.New if any type needing comparison has
// either a custom String method or Equal method (Decimal for example)
type CustomRequire struct {
	t       *testing.T
	options cmp.Options // This is a []Option
}

func NewCustomRequire(t *testing.T) *CustomRequire {
	return &CustomRequire{t, []cmp.Option{
		cmp.Comparer(TxTestEqual),
	}}
}

func (rq *CustomRequire) PanicsWithRegexp(regex interface{}, fn func()) {
	RqPanicsWithRegexp(rq.t, regex, fn)
}

func (rq *CustomRequire) Equal(expected, actual interface{}) {
	diff := cmp.Diff(expected, actual, rq.options)
	require.True(rq.t, diff == "", diff)
}

func (rq *CustomRequire) LinesEqual(expected, actual string) {
	expLines := strings.Split(expected, "\n")
	actLines := strings.Split(actual, "\n")
	diff := cmp.Diff(expLines, actLines, rq.options)
	require.True(rq.t, diff == "", diff)
}
