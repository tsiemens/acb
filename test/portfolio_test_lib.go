package test

import (
	"fmt"
	"math"
	"regexp"
	"strings"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	ptf "github.com/tsiemens/acb/portfolio"
	"github.com/tsiemens/acb/util"
)

const DefaultTestSecurity string = "FOO"

func mkDateYD(year uint32, day int) date.Date {
	tm := date.New(year, time.January, 1)
	return tm.AddDays(day)
}

func mkDate(day int) date.Date {
	return mkDateYD(2017, day)
}

func CADSFL(lossVal float64, force bool) util.Optional[ptf.SFLInput] {
	util.Assert(lossVal <= 0.0)
	return util.NewOptional[ptf.SFLInput](ptf.SFLInput{lossVal, force})
}

func addTx(tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) (*ptf.TxDelta, []*ptf.Tx, error) {
	txs := []*ptf.Tx{tx}
	affil := ptf.NonNilTxAffiliate(tx)
	ptfStatuses := ptf.NewAffiliatePortfolioSecurityStatuses(tx.Security, nil)
	shareDiff := preTxStatus.AllAffiliatesShareBalance - preTxStatus.ShareBalance
	// Set up the previous balance to avoid assert
	dummyId := ptf.GlobalAffiliateDedupTable.DedupedAffiliate("dummy").Id()
	ptfStatuses.SetLatestPostStatus(dummyId, TPSS{Shares: shareDiff}.X())
	ptfStatuses.SetLatestPostStatus(affil.Id(), preTxStatus)
	return ptf.AddTx(0, txs, ptfStatuses)
}

// **********************************************************************************
// Test Types/Models
// **********************************************************************************

// Using DEFAULT_CURRENCY in TTx will just result in CAD.
// If testing actual DEFAULT_CURRENCY, use this.
const EXP_DEFAULT_CURRENCY ptf.Currency = "EXPLICIT_TEST_DEFAULT_CURRENCY"
const EXP_FLOAT_ZERO = -0.01010101

// Test Tx
type TTx struct {
	Sec        string
	TDay       int       // An abitrarily offset day. Convenience for TDate
	TDate      date.Date // Defaults to 2 days before SDate
	SYr        uint32    // Year. Convenience for SDate. Must be combined with TDoY
	SDoY       int       // Day of Year. Convenience for SDate. Must be combined with TYr
	SDate      date.Date // Defaults to 2 days after TDate/TDay
	Act        ptf.TxAction
	Shares     uint32
	Price      float64
	Comm       float64
	Curr       ptf.Currency
	FxRate     float64
	CommCurr   ptf.Currency
	CommFxRate float64
	Memo       string
	Affiliate  *ptf.Affiliate
	AffName    string
	SFL        util.Optional[ptf.SFLInput]
	ReadIndex  uint32
}

// eXpand to full type.
func (t TTx) X() *ptf.Tx {
	getFxRate := func(rateArg float64, def float64) float64 {
		if rateArg == 0.0 {
			return def
		} else if rateArg == EXP_FLOAT_ZERO {
			return 0.0
		}
		return rateArg
	}
	fxRate := getFxRate(t.FxRate, 1.0)
	affiliate := t.Affiliate
	if affiliate == nil {
		affiliate = ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)
	} else {
		util.Assert(t.AffName == "")
	}

	// Dates
	tradeDate := util.Tern(t.TDay != 0, mkDate(t.TDay), t.TDate)
	if t.TDay != 0 {
		util.Assert(t.TDate == date.Date{})
	}
	settlementDate := util.Tern(t.SYr != 0, mkDateYD(t.SYr, t.SDoY), t.SDate)
	if t.SYr != 0 || t.SDoY != 0 {
		util.Assert(t.SDate == date.Date{})
	}
	if (settlementDate == date.Date{}) && (tradeDate != date.Date{}) {
		settlementDate = tradeDate.AddDays(2)
	} else if (tradeDate == date.Date{}) && (settlementDate != date.Date{}) {
		tradeDate = settlementDate.AddDays(-2)
	}

	getCurr := func(specifiedCurr ptf.Currency, default_ ptf.Currency) ptf.Currency {
		curr := specifiedCurr
		if curr == "" {
			util.Assert(curr == ptf.DEFAULT_CURRENCY)
			curr = default_
		} else if curr == EXP_DEFAULT_CURRENCY {
			curr = ptf.DEFAULT_CURRENCY
		}
		return curr
	}
	curr := getCurr(t.Curr, ptf.CAD)
	commCurr := getCurr(t.CommCurr, curr)

	return &ptf.Tx{
		Security:                          util.Tern(t.Sec == "", DefaultTestSecurity, t.Sec),
		TradeDate:                         tradeDate,
		SettlementDate:                    settlementDate,
		Action:                            t.Act,
		Shares:                            t.Shares,
		AmountPerShare:                    t.Price,
		Commission:                        t.Comm,
		TxCurrency:                        curr,
		TxCurrToLocalExchangeRate:         fxRate,
		CommissionCurrency:                commCurr,
		CommissionCurrToLocalExchangeRate: getFxRate(t.CommFxRate, fxRate),
		Memo:                              t.Memo,
		Affiliate:                         affiliate,

		SpecifiedSuperficialLoss: t.SFL,

		ReadIndex: t.ReadIndex,
	}
}

// Test PortfolioSecurityStatus
type TPSS struct {
	Sec       string
	Shares    uint32
	AllShares uint32
	TotalAcb  float64
	AcbPerSh  float64
}

// // eXpand to full type.
func (o TPSS) X() *ptf.PortfolioSecurityStatus {
	util.Assert(!(o.TotalAcb != 0.0 && o.AcbPerSh != 0.0))

	return &ptf.PortfolioSecurityStatus{
		Security:                  util.Tern(o.Sec == "", DefaultTestSecurity, o.Sec),
		ShareBalance:              o.Shares,
		AllAffiliatesShareBalance: util.Tern(o.AllShares > 0, o.AllShares, o.Shares),
		TotalAcb:                  util.Tern(o.AcbPerSh != 0.0, o.AcbPerSh*float64(o.Shares), o.TotalAcb),
	}
}

// Test Delta
type TDt struct {
	PostSt                    TPSS
	Gain                      float64
	SFL                       float64
	PotentiallyOverAppliedSfl bool
}

// **********************************************************************************
// Validation functions
// **********************************************************************************

// This will represent NaN, but allow us to actually perform an equality check
const nanVal float64 = -123.0123456789

const matchingMemoPrefix string = "TEST_MEMO_MATCHES:"

func matchingMemo(pattern string) string {
	return matchingMemoPrefix + pattern
}

func SoftTxEq(t *testing.T, exp *ptf.Tx, actual *ptf.Tx) bool {
	var expCopy ptf.Tx = *exp
	var actualCopy ptf.Tx = *actual

	if math.IsNaN(expCopy.AmountPerShare) {
		// Allow us to do equality check with NaN
		expCopy.AmountPerShare = nanVal
	}
	if math.IsNaN(actualCopy.AmountPerShare) {
		actualCopy.AmountPerShare = nanVal
	}
	if IsAlmostEqual(actualCopy.AmountPerShare, expCopy.AmountPerShare) {
		expCopy.AmountPerShare = actualCopy.AmountPerShare
	}
	// To match the memo using a regex, set the expected memo with matchingMemo()
	if strings.HasPrefix(expCopy.Memo, matchingMemoPrefix) {
		pattern := strings.TrimPrefix(expCopy.Memo, matchingMemoPrefix)
		if regexp.MustCompile(pattern).MatchString(actualCopy.Memo) {
			expCopy.Memo = actualCopy.Memo
		}
	}

	return assert.Equal(t, expCopy, actualCopy)
}

func ValidateTxs(t *testing.T, expTxs []*ptf.Tx, actualTxs []*ptf.Tx) {
	if !assert.Equal(t, len(expTxs), len(actualTxs)) {
		for j, _ := range actualTxs {
			fmt.Println(j, "Tx:", actualTxs[j], "Af:", actualTxs[j].Affiliate.Id())
		}
		require.FailNow(t, "ValidateTxs failed")
	}
	for i, tx := range actualTxs {
		fail := false
		fail = !SoftTxEq(t, expTxs[i], tx) || fail
		if fail {
			for j, _ := range actualTxs {
				fmt.Println(j, "Tx:", actualTxs[j], "Af:", actualTxs[j].Affiliate.Id())
			}
			require.FailNowf(t, "ValidateTxs failed", "Tx %d", i)
		}
	}
}

func SoftStEq(
	t *testing.T,
	exp *ptf.PortfolioSecurityStatus, actual *ptf.PortfolioSecurityStatus) bool {

	var expCopy ptf.PortfolioSecurityStatus = *exp
	var actualCopy ptf.PortfolioSecurityStatus = *actual

	if math.IsNaN(expCopy.TotalAcb) {
		// Allow us to do equality check with NaN
		expCopy.TotalAcb = nanVal
	}
	if math.IsNaN(actualCopy.TotalAcb) {
		actualCopy.TotalAcb = nanVal
	}

	// For the sake of sanity, allow ourselves to specify approximate float values.
	if IsAlmostEqual(expCopy.TotalAcb, actualCopy.TotalAcb) {
		expCopy.TotalAcb = actualCopy.TotalAcb
	}

	return assert.Equal(t, expCopy, actualCopy)
}

func StEq(
	t *testing.T,
	exp *ptf.PortfolioSecurityStatus, actual *ptf.PortfolioSecurityStatus) {
	if !SoftStEq(t, exp, actual) {
		t.FailNow()
	}
}

func SoftSflAlmostEqual(t *testing.T, expDelta TDt, delta *ptf.TxDelta) bool {
	if expDelta.SFL != 0.0 {
		if expDelta.SFL == EXP_FLOAT_ZERO {
			return SoftAlmostEqual(t, 0.0, delta.SuperficialLoss)
		} else {
			return SoftAlmostEqual(t, expDelta.SFL, delta.SuperficialLoss)
		}
	}
	return true
}

func ValidateDelta(t *testing.T, delta *ptf.TxDelta, expDelta TDt) {
	fail := false
	fail = !SoftStEq(t, expDelta.PostSt.X(), delta.PostStatus) || fail
	fail = !SoftAlmostEqual(t, expDelta.Gain, delta.CapitalGain) || fail
	fail = !SoftSflAlmostEqual(t, expDelta, delta) || fail
	if fail {
		require.FailNow(t, "ValidateDelta failed")
	}
}

func ValidateDeltas(t *testing.T, deltas []*ptf.TxDelta, expDeltas []TDt) {
	if len(expDeltas) != len(deltas) {
		for j, _ := range deltas {
			fmt.Println(j, "Tx:", deltas[j].Tx, "PostStatus:", deltas[j].PostStatus)
		}
		require.Equal(t, len(expDeltas), len(deltas), "Num deltas did not match")
	}
	for i, delta := range deltas {
		fail := false
		fail = !SoftStEq(t, expDeltas[i].PostSt.X(), delta.PostStatus) || fail
		fail = !SoftAlmostEqual(t, expDeltas[i].Gain, delta.CapitalGain,
			"Capital Gains were not almost equal") || fail
		fail = !SoftSflAlmostEqual(t, expDeltas[i], delta) || fail
		fail = (expDeltas[i].PotentiallyOverAppliedSfl != delta.PotentiallyOverAppliedSfl) || fail
		if fail {
			for j, _ := range deltas {
				fmt.Println(j, "Tx:", deltas[j].Tx, "PostStatus:", deltas[j].PostStatus,
					"Gain:", deltas[j].CapitalGain, "SFL:", deltas[j].SuperficialLoss,
					"PotentiallyOverAppliedSfl:", deltas[j].PotentiallyOverAppliedSfl)
			}
			require.FailNowf(t, "ValidateDeltas failed", "Delta %d", i)
		}
	}
}
