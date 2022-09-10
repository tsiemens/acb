package test

import (
	"testing"

	"github.com/stretchr/testify/require"

	ptf "github.com/tsiemens/acb/portfolio"
	"github.com/tsiemens/acb/util"
)

func ensureAffiliates() (defaultAfId, defaultRAfId, afIdB, afIdBR, afIdC, afIdCR string) {
	defaultAfId = ptf.GlobalAffiliateDedupTable.DedupedAffiliate("Default").Id()
	defaultRAfId = ptf.GlobalAffiliateDedupTable.DedupedAffiliate("Default (R)").Id()
	afIdB = ptf.GlobalAffiliateDedupTable.DedupedAffiliate("B").Id()
	afIdBR = ptf.GlobalAffiliateDedupTable.DedupedAffiliate("B (R)").Id()
	afIdC = ptf.GlobalAffiliateDedupTable.DedupedAffiliate("C").Id()
	afIdCR = ptf.GlobalAffiliateDedupTable.DedupedAffiliate("C (R)").Id()
	return
}

func TestAffiliatePortfolioSecurityStatusesBasic(t *testing.T) {
	rq := require.New(t)
	// TODO

	// defaultAfId, defaultRAfId, afIdB, afIdBR, afIdC, afIdCR := ensureAffiliates()
	defaultAfId, _, afIdB, _, _, _ := ensureAffiliates()

	// Case:
	// GetLatestPostStatusForAffiliate("default")
	// GetLatestPostStatusForAffiliate("B")
	statuses := ptf.NewAffiliatePortfolioSecurityStatuses(DefaultTestSecurity, nil)
	defaultPss, ok := statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.Nil(defaultPss)
	rq.False(ok)
	pssB, ok := statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.False(ok)
	rq.Nil(pssB)

	// Case:
	// (initial default state)
	// GetLatestPostStatusForAffiliate("default")
	// GetLatestPostStatusForAffiliate("B")
	nonDefaultInitStatus := TPSS{Shares: 12, TotalAcb: 24.0}.X()
	statuses = ptf.NewAffiliatePortfolioSecurityStatuses(
		DefaultTestSecurity, nonDefaultInitStatus)
	defaultPss, ok = statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.True(ok)
	StEq(t, TPSS{Shares: 12, TotalAcb: 24.0}.X(), defaultPss)
	pssB, ok = statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.False(ok)
	rq.Nil(pssB)
}

func TestAffiliatePortfolioSecurityStatusesGetLatest(t *testing.T) {
	// defaultAfId, defaultRAfId, afIdB, afIdBR, afIdC, afIdCR := ensureAffiliates()
	_, _, afIdB, _, _, _ := ensureAffiliates()

	// Case:
	// GetLatestPostStatus()
	statuses := ptf.NewAffiliatePortfolioSecurityStatuses(DefaultTestSecurity, nil)
	latest := statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 0, TotalAcb: 0.0}.X(), latest)

	// Case:
	// (init with default)
	// GetLatestPostStatus()
	nonDefaultInitStatus := TPSS{Shares: 12, TotalAcb: 24.0}.X()
	statuses = ptf.NewAffiliatePortfolioSecurityStatuses(
		DefaultTestSecurity, nonDefaultInitStatus)
	latest = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 12, TotalAcb: 24.0}.X(), latest)

	// Case:
	// SetLatestPostStatus("B")
	// GetLatestPostStatus()
	statuses = ptf.NewAffiliatePortfolioSecurityStatuses(DefaultTestSecurity, nil)
	statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 2, TotalAcb: 4.0}.X())
	latest = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 2, TotalAcb: 4.0}.X(), latest)

	util.AssertsPanic = true

	// Case:
	// (init with default)
	// SetLatestPostStatus("B") // invalid all share bal
	// GetLatestPostStatus()
	statuses = ptf.NewAffiliatePortfolioSecurityStatuses(
		DefaultTestSecurity, nonDefaultInitStatus)
	RqPanicsWithRegexp(t, "AllAffiliatesShareBalance", func() {
		statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 2, TotalAcb: 4.0}.X())
	})

	// Case:
	// (init with default)
	// SetLatestPostStatus("B")
	// GetLatestPostStatus()
	statuses = ptf.NewAffiliatePortfolioSecurityStatuses(
		DefaultTestSecurity, nonDefaultInitStatus)
	statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 2, AllShares: 14, TotalAcb: 4.0}.X())
	latest = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 2, AllShares: 14, TotalAcb: 4.0}.X(), latest)
}

func TestAffiliatePortfolioSecurityStatusesGetNextPreGetLatest(t *testing.T) {
	// defaultAfId, defaultRAfId, afIdB, afIdBR, afIdC, afIdCR := ensureAffiliates()
	defaultAfId, _, afIdB, _, _, _ := ensureAffiliates()

	// Case:
	// SetLatestPostStatus("B")
	// GetNextPreStatus("Default")
	// GetLatestPostStatus()
	statuses := ptf.NewAffiliatePortfolioSecurityStatuses(DefaultTestSecurity, nil)
	statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 2, AllShares: 2, TotalAcb: 4.0}.X())
	defaultStatus := statuses.GetNextPreStatus(defaultAfId)
	StEq(t, TPSS{Shares: 0, AllShares: 2, TotalAcb: 0.0}.X(), defaultStatus)
	latest := statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 2, AllShares: 2, TotalAcb: 4.0}.X(), latest)

	// Case:
	// (init with default)
	// SetLatestPostStatus("B")
	// GetNextPreStatus("Default")
	// GetLatestPostStatus()
	nonDefaultInitStatus := TPSS{Shares: 12, TotalAcb: 24.0}.X()
	statuses = ptf.NewAffiliatePortfolioSecurityStatuses(
		DefaultTestSecurity, nonDefaultInitStatus)
	statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 2, AllShares: 14, TotalAcb: 4.0}.X())
	defaultStatus = statuses.GetNextPreStatus(defaultAfId)
	StEq(t, TPSS{Shares: 12, AllShares: 14, TotalAcb: 24.0}.X(), defaultStatus)
	latest = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 2, AllShares: 14, TotalAcb: 4.0}.X(), latest)
}

func TestAffiliatePortfolioSecurityStatusesFullUseCase(t *testing.T) {
	util.AssertsPanic = true
	rq := require.New(t)
	// defaultAfId, defaultRAfId, afIdB, afIdBR, afIdC, afIdCR := ensureAffiliates()
	defaultAfId, _, afIdB, _, _, _ := ensureAffiliates()

	// Case:
	// GetNextPreStatus("Default")
	// Get*
	// SetLatestPostStatus("Default")
	//
	// GetNextPreStatus("Default")
	// Get*
	// SetLatestPostStatus("Default")
	//
	// GetNextPreStatus("B")
	// Get*
	// SetLatestPostStatus("B")
	//
	// GetNextPreStatus("B")
	// Get*
	// SetLatestPostStatus("B")
	//
	// GetNextPreStatus("Default")
	// Get*
	// SetLatestPostStatus("Default")

	// Buy 2 default
	statuses := ptf.NewAffiliatePortfolioSecurityStatuses(DefaultTestSecurity, nil)
	nextPre := statuses.GetNextPreStatus(defaultAfId)
	StEq(t, TPSS{Shares: 0, AllShares: 0, TotalAcb: 0.0}.X(), nextPre)
	latestPost := statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 0, AllShares: 0, TotalAcb: 0.0}.X(), latestPost)
	latestDef, ok := statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.False(ok)
	latestB, ok := statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.False(ok)
	statuses.SetLatestPostStatus(defaultAfId, TPSS{Shares: 2, TotalAcb: 4.0}.X())

	// Buy 1 default
	nextPre = statuses.GetNextPreStatus(defaultAfId)
	StEq(t, TPSS{Shares: 2, TotalAcb: 4.0}.X(), nextPre)
	latestPost = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 2, AllShares: 2, TotalAcb: 4.0}.X(), latestPost)
	latestDef, ok = statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.True(ok)
	StEq(t, TPSS{Shares: 2, AllShares: 2, TotalAcb: 4.0}.X(), latestDef)
	latestB, ok = statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.False(ok)
	statuses.SetLatestPostStatus(defaultAfId, TPSS{Shares: 3, TotalAcb: 6.0}.X())

	// Buy 12 B
	nextPre = statuses.GetNextPreStatus(afIdB)
	StEq(t, TPSS{Shares: 0, AllShares: 3, TotalAcb: 0.0}.X(), nextPre)
	latestPost = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 3, AllShares: 3, TotalAcb: 6.0}.X(), latestPost)
	latestDef, ok = statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.True(ok)
	StEq(t, TPSS{Shares: 3, AllShares: 3, TotalAcb: 6.0}.X(), latestDef)
	latestB, ok = statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.False(ok)
	RqPanicsWithRegexp(t, "AllAffiliatesShareBalance", func() {
		statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 12, AllShares: 12, TotalAcb: 24.0}.X())
	})
	statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 12, AllShares: 15, TotalAcb: 24.0}.X())

	// Sell 6 B
	nextPre = statuses.GetNextPreStatus(afIdB)
	StEq(t, TPSS{Shares: 12, AllShares: 15, TotalAcb: 24.0}.X(), nextPre)
	latestPost = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 12, AllShares: 15, TotalAcb: 24.0}.X(), latestPost)
	latestDef, ok = statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.True(ok)
	StEq(t, TPSS{Shares: 3, AllShares: 3, TotalAcb: 6.0}.X(), latestDef)
	latestB, ok = statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.True(ok)
	StEq(t, TPSS{Shares: 12, AllShares: 15, TotalAcb: 24.0}.X(), latestB)
	RqPanicsWithRegexp(t, "AllAffiliatesShareBalance", func() {
		statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 6, AllShares: 15, TotalAcb: 24.0}.X())
	})
	statuses.SetLatestPostStatus(afIdB, TPSS{Shares: 6, AllShares: 9, TotalAcb: 12.0}.X())

	// Buy 1 default
	nextPre = statuses.GetNextPreStatus(defaultAfId)
	StEq(t, TPSS{Shares: 3, AllShares: 9, TotalAcb: 6.0}.X(), nextPre)
	latestPost = statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 6, AllShares: 9, TotalAcb: 12.0}.X(), latestPost)
	latestDef, ok = statuses.GetLatestPostStatusForAffiliate(defaultAfId)
	rq.True(ok)
	StEq(t, TPSS{Shares: 3, AllShares: 3, TotalAcb: 6.0}.X(), latestDef)
	latestB, ok = statuses.GetLatestPostStatusForAffiliate(afIdB)
	rq.True(ok)
	StEq(t, TPSS{Shares: 6, AllShares: 9, TotalAcb: 12.0}.X(), latestB)
	statuses.SetLatestPostStatus(defaultAfId, TPSS{Shares: 4, AllShares: 10, TotalAcb: 6.0}.X())
}

func TestAffiliatePortfolioSecurityStatusRegistered(t *testing.T) {
	util.AssertsPanic = true
	// rq := require.New(t)
	// defaultAfId, defaultRAfId, afIdB, afIdBR, afIdC, afIdCR := ensureAffiliates()
	defaultAfId, defRAfId, _, _, _, _ := ensureAffiliates()

	statuses := ptf.NewAffiliatePortfolioSecurityStatuses(DefaultTestSecurity, nil)

	// Case:
	// GetNextPreStatus("(R)")
	nextPre := statuses.GetNextPreStatus(defRAfId)
	StEq(t, TPSS{Shares: 0, AllShares: 0, TotalAcb: NaN}.X(), nextPre)

	// Case:
	// SetLatestPostStatus("(R)")
	statuses.SetLatestPostStatus(defRAfId, TPSS{Shares: 1, AllShares: 1, TotalAcb: NaN}.X())
	latestPost := statuses.GetLatestPostStatus()
	StEq(t, TPSS{Shares: 1, AllShares: 1, TotalAcb: NaN}.X(), latestPost)

	// Case:
	// SetLatestPostStatus("(R)") // non-NaN values (panics)
	RqPanicsWithRegexp(t, "bad NaN value", func() {
		statuses.SetLatestPostStatus(defRAfId, TPSS{Shares: 0, AllShares: 0, TotalAcb: 0.0}.X())
	})

	// Case:
	// SetLatestPostStatus("default") // NaN values (panics)
	RqPanicsWithRegexp(t, "bad NaN value", func() {
		statuses.SetLatestPostStatus(defaultAfId, TPSS{Shares: 0, AllShares: 0, TotalAcb: NaN}.X())
	})
}
