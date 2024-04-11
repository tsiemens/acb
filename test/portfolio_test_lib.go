package test

import (
	"fmt"
	"regexp"
	"strings"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	decimal "github.com/tsiemens/acb/decimal_value"
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

func CADSFL(lossVal decimal.Decimal, force bool) util.Optional[ptf.SFLInput] {
	util.Assert(lossVal.LessThanOrEqual(decimal.Zero))
	return util.NewOptional[ptf.SFLInput](ptf.SFLInput{lossVal, force})
}

func addTx(tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) (*ptf.TxDelta, []*ptf.Tx, error) {
	txs := []*ptf.Tx{tx}
	affil := ptf.NonNilTxAffiliate(tx)
	ptfStatuses := ptf.NewAffiliatePortfolioSecurityStatuses(tx.Security, nil)
	shareDiff := preTxStatus.AllAffiliatesShareBalance.Sub(preTxStatus.ShareBalance)
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
	Shares     decimal.Decimal
	Price      decimal.Decimal
	Comm       decimal.Decimal
	Curr       ptf.Currency
	FxRate     decimal.Decimal
	CommCurr   ptf.Currency
	CommFxRate decimal.Decimal
	Memo       string
	Affiliate  *ptf.Affiliate
	AffName    string
	SFL        util.Optional[ptf.SFLInput]
	ReadIndex  uint32
}

// eXpand to full type.
func (t TTx) X() *ptf.Tx {
	getFxRate := func(rateArg decimal.Decimal, def decimal.Decimal) decimal.Decimal {
		if rateArg.IsZero() {
			return def
		}
		return rateArg
	}
	fxRate := getFxRate(t.FxRate, decimal.NewFromInt(1))
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
	Shares    decimal.Decimal
	AllShares decimal.Decimal
	TotalAcb  decimal.Decimal
	AcbPerSh  decimal.Decimal
}

// // eXpand to full type.
func (o TPSS) X() *ptf.PortfolioSecurityStatus {
	util.Assert(!(!o.TotalAcb.IsZero() && !o.AcbPerSh.IsZero()))

	return &ptf.PortfolioSecurityStatus{
		Security:                  util.Tern(o.Sec == "", DefaultTestSecurity, o.Sec),
		ShareBalance:              o.Shares,
		AllAffiliatesShareBalance: util.Tern(o.AllShares.IsPositive(), o.AllShares, o.Shares),
		TotalAcb:                  util.Tern(o.AcbPerSh.IsZero(), o.TotalAcb, o.AcbPerSh.Mul(o.Shares)),
	}
}

// Test Delta
type TDt struct {
	PostSt                    TPSS
	Gain                      decimal.Decimal
	SFL                       decimal.Decimal
	PotentiallyOverAppliedSfl bool
}

// **********************************************************************************
// Validation functions
// **********************************************************************************

const matchingMemoPrefix string = "TEST_MEMO_MATCHES:"

func matchingMemo(pattern string) string {
	return matchingMemoPrefix + pattern
}

func SoftTxEq(t *testing.T, exp *ptf.Tx, actual *ptf.Tx) bool {
	var expCopy ptf.Tx = *exp
	var actualCopy ptf.Tx = *actual

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

func ValidateDelta(t *testing.T, delta *ptf.TxDelta, expDelta TDt) {
	fail := false
	fail = !assert.Equal(t, expDelta.PostSt.X(), delta.PostStatus) || fail
	fail = !expDelta.Gain.Equal(delta.CapitalGain) || fail
	fail = !expDelta.SFL.Equal(delta.SuperficialLoss) || fail
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
		fail = !assert.Equal(t, expDeltas[i].PostSt.X(), delta.PostStatus) || fail
		fail = !expDeltas[i].Gain.Equal(delta.CapitalGain) || fail
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
