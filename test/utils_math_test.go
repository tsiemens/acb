package test

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/util"
)

func TestMathMin(t *testing.T) {
	require.Equal(t, util.MinUint32(50, 40), uint32(40))
	require.Equal(t, util.MinUint32(40, 50, 60), uint32(40))
	require.Equal(t, util.MinUint32(60, 50, 40), uint32(40))
}
