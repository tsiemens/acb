package test

import (
	"fmt"
	"math"
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

func addTx(tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) (*ptf.TxDelta, *ptf.Tx, error) {
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
	TDay       int // Convenience for TDate
	TDate      date.Date
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

	tradeDate := util.Tern(t.TDay != 0, mkDate(t.TDay), t.TDate)
	if t.TDay != 0 {
		util.Assert(t.TDate == date.Date{})
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
		SettlementDate:                    util.Tern(t.SDate == date.Date{}, tradeDate.AddDays(2), t.SDate),
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
	PostSt TPSS
	Gain   float64
	SFL    float64
}

// **********************************************************************************
// Validation functions
// **********************************************************************************

func SoftStEq(
	t *testing.T,
	exp *ptf.PortfolioSecurityStatus, actual *ptf.PortfolioSecurityStatus) bool {

	var expCopy ptf.PortfolioSecurityStatus = *exp
	var actualCopy ptf.PortfolioSecurityStatus = *actual

	// This will represent NaN, but allow us to actually perform an equality check
	nanVal := -123.0123456789
	if math.IsNaN(expCopy.TotalAcb) {
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
	require.Equal(t, len(expDeltas), len(deltas))
	for i, delta := range deltas {
		fail := false
		fail = !SoftStEq(t, expDeltas[i].PostSt.X(), delta.PostStatus) || fail
		fail = !SoftAlmostEqual(t, expDeltas[i].Gain, delta.CapitalGain,
			"Capital Gains were not almost equal") || fail
		fail = !SoftSflAlmostEqual(t, expDeltas[i], delta) || fail
		if fail {
			for j, _ := range deltas {
				fmt.Println(j, "Tx:", deltas[j].Tx, "PostStatus:", deltas[j].PostStatus)
			}
			require.FailNowf(t, "ValidateDeltas failed", "Delta %d", i)
		}
	}
}
