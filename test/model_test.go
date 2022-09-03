package test

import (
	"testing"

	"github.com/stretchr/testify/require"

	ptf "github.com/tsiemens/acb/portfolio"
)

func TestNewAffiliate(t *testing.T) {
	rq := require.New(t)

	newAffiliate := func(name string) ptf.Affiliate {
		return ptf.NewUndedupedAffiliate(name)
	}
	verifyAffiliate := func(name string,
		expId string, expName string, expRegistered bool) {
		af := newAffiliate(name)
		rq.Equal(expId, af.Id())
		rq.Equal(expName, af.Name())
		rq.Equal(expRegistered, af.Registered())
	}

	rq.Equal(newAffiliate(""), newAffiliate(""))
	verifyAffiliate("", "default", "Default", false)
	verifyAffiliate("  ", "default", "Default", false)
	verifyAffiliate("  default", "default", "default", false)
	verifyAffiliate("  Default", "default", "Default", false)

	verifyAffiliate(" (r) ", "default (R)", "Default (R)", true)
	verifyAffiliate("(R)", "default (R)", "Default (R)", true)
	verifyAffiliate("default(R)", "default (R)", "default (R)", true)
	verifyAffiliate("(R)Default", "default (R)", "Default (R)", true)
	verifyAffiliate("(R)Default(r)", "default (R)", "Default (R)", true)
	verifyAffiliate("Def(r)ault", "def ault (R)", "Def ault (R)", true)

	verifyAffiliate(" My Spouse ", "my spouse", "My Spouse", false)
	verifyAffiliate(" My     Spouse ", "my spouse", "My Spouse", false)
	verifyAffiliate(" My  (r)   Spouse ", "my spouse (R)", "My Spouse (R)", true)
}

func TestAffiliateDedupTable(t *testing.T) {
	rq := require.New(t)

	dt := ptf.NewAffiliateDedupTable()

	// Check basic deduping for one entry
	af1 := dt.DedupedAffiliate("")
	rq.Equal(ptf.NewUndedupedAffiliate("Default"), *af1)
	af2 := dt.DedupedAffiliate("  Default  ")
	af3 := dt.DedupedAffiliate("default")
	rq.Equal(af1, af2)
	rq.Equal(af1, af3)

	// Check that a different entry dedupes differently
	af4 := dt.DedupedAffiliate("(R)")
	rq.NotEqual(af1, af4)

	// Check that the first entry is still retained in the dedup table
	rq.Equal(af1, dt.DedupedAffiliate("default"))
}
