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

var NaN float64 = math.NaN()

func mkDateYD(year uint32, day int) date.Date {
	tm := date.New(year, time.January, 1)
	return tm.AddDays(day)
}

func mkDate(day int) date.Date {
	return mkDateYD(2017, day)
}

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

func AddTxNoErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) *ptf.TxDelta {
	txs := []*ptf.Tx{tx}
	delta, newTx, err := ptf.AddTx(0, txs, preTxStatus)
	require.Nil(t, newTx)
	require.Nil(t, err)
	return delta
}

func AddTxWithErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) error {
	txs := []*ptf.Tx{tx}
	delta, newTx, err := ptf.AddTx(0, txs, preTxStatus)
	require.NotNil(t, err)
	require.Nil(t, delta)
	require.Nil(t, newTx)
	return err
}

type CurrOpt = util.Optional[ptf.Currency]

// Test Tx
type TTx struct {
	Sec          string
	TDate        date.Date
	SDate        date.Date
	Act          ptf.TxAction
	Shares       uint32
	Price        float64
	Comm         float64
	Currency     CurrOpt
	FxRate       float64
	CommCurrency CurrOpt
	CommFxRate   float64
	Memo         string
	Affiliate    *ptf.Affiliate
	AffName      string
	SFL          util.Optional[ptf.SFLInput]
	ReadIndex    uint32
}

// eXpand to full type.
func (t TTx) X() *ptf.Tx {
	fxRate := util.Tern(t.FxRate == 0.0, 1.0, t.FxRate)
	affiliate := t.Affiliate
	if affiliate == nil {
		affiliate = ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)
	} else {
		util.Assert(t.AffName == "")
	}

	return &ptf.Tx{
		Security:                          util.Tern(t.Sec == "", DefaultTestSecurity, t.Sec),
		TradeDate:                         t.TDate,
		SettlementDate:                    util.Tern(t.SDate == date.Date{}, t.TDate.AddDays(2), t.SDate),
		Action:                            t.Act,
		Shares:                            t.Shares,
		AmountPerShare:                    t.Price,
		Commission:                        t.Comm,
		TxCurrency:                        t.Currency.GetOr(ptf.CAD),
		TxCurrToLocalExchangeRate:         util.Tern(t.FxRate == 0.0, 1.0, t.FxRate),
		CommissionCurrency:                t.CommCurrency.GetOr(t.Currency.GetOr(ptf.CAD)),
		CommissionCurrToLocalExchangeRate: util.Tern(t.CommFxRate == 0.0, fxRate, t.CommFxRate),
		Memo:                              t.Memo,
		Affiliate:                         affiliate,

		SpecifiedSuperficialLoss: t.SFL,

		ReadIndex: t.ReadIndex,
	}
}

type SimplePtfSecSt struct {
	Security string
	Shares   uint32
	TotalAcb float64
}

// eXpand to full type.
func (o SimplePtfSecSt) X() *ptf.PortfolioSecurityStatus {
	return &ptf.PortfolioSecurityStatus{
		Security:                  o.Security,
		ShareBalance:              o.Shares,
		AllAffiliatesShareBalance: o.Shares,
		TotalAcb:                  o.TotalAcb,
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

func TestBasicBuyAcb(t *testing.T) {
	rq := require.New(t)

	sptf := ptf.NewEmptyPortfolioSecurityStatus("FOO")
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.BUY,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 3, TotalAcb: 30.0}.X(),
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with commission
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.BUY,
		Shares: 2, AmountPerShare: 10.0, Commission: 1.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 21.0}.X(),
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with exchange rates
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.BUY,
		Shares: 3, AmountPerShare: 12.0, Commission: 1.0,
		TxCurrency: ptf.USD, TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: "XXX", CommissionCurrToLocalExchangeRate: 0.3}

	delta = AddTxNoErr(t, tx, delta.PostStatus)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 5,
			TotalAcb: 21.0 + (2 * 36.0) + 0.3}.X(),
	)
	rq.Equal(delta.CapitalGain, 0.0)
}

func TestBasicSellAcbErrors(t *testing.T) {
	sptf := SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 20.0}.X()
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	AddTxWithErr(t, tx, sptf)
}

func TestBasicSellAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 20.0}.X()
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 0, TotalAcb: 0.0}.X(),
	)
	rq.Equal(delta.CapitalGain, 10.0)

	// Sell shares with commission
	sptf = SimplePtfSecSt{Security: "FOO", Shares: 3, TotalAcb: 30.0}.X()
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 1.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 1, TotalAcb: 10.0}.X(),
	)
	rq.Equal(delta.CapitalGain, 9.0)

	// Sell shares with exchange rate
	sptf = SimplePtfSecSt{Security: "FOO", Shares: 3, TotalAcb: 30.0}.X()
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 2.0,
		TxCurrency: "XXX", TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: "YYY", CommissionCurrToLocalExchangeRate: 0.4}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 1, TotalAcb: 10.0}.X(),
	)
	rq.Equal(delta.CapitalGain, (15.0*2.0*2.0)-20.0-0.8)
}

func TestSuperficialLosses(t *testing.T) {
	rq := require.New(t)

	makeTxV := func(
		day int, action ptf.TxAction, shares uint32, amount float64,
		curr ptf.Currency, fxRate float64, expSfl util.Optional[ptf.SFLInput],
	) *ptf.Tx {
		commission := 0.0
		if action == ptf.BUY {
			commission = 2.0
		}
		return &ptf.Tx{Security: "FOO", SettlementDate: mkDate(day), Action: action,
			Shares: shares, AmountPerShare: amount, Commission: commission,
			TxCurrency: curr, TxCurrToLocalExchangeRate: fxRate,
			CommissionCurrency: curr, CommissionCurrToLocalExchangeRate: fxRate,
			SpecifiedSuperficialLoss: expSfl,
		}
	}
	makeTxWithCurr := func(
		day int, action ptf.TxAction, shares uint32, amount float64,
		curr ptf.Currency, fxRate float64,
	) *ptf.Tx {
		return makeTxV(day, action, shares, amount, curr, fxRate, util.Optional[ptf.SFLInput]{})
	}
	makeTx := func(day int, action ptf.TxAction, shares uint32, amount float64) *ptf.Tx {
		return makeTxWithCurr(day, action, shares, amount, ptf.CAD, 1.0)
	}

	/*
		buy 10
		wait
		sell 5 (loss, not superficial)
	*/
	tx0 := makeTx(1, ptf.BUY, 10, 1.0)
	// Sell half at a loss a while later, for a total of $1
	tx1 := makeTx(50, ptf.SELL, 5, 0.2)
	txs := []*ptf.Tx{tx0, tx1}

	var deltas []*ptf.TxDelta
	var err error

	validate := func(i int, shareBalance uint32, totalAcb float64, gain float64) {
		AlmostEqual(t, totalAcb, deltas[i].PostStatus.TotalAcb)
		rq.Equal(
			SimplePtfSecSt{
				Security: "FOO",
				Shares:   shareBalance,
				TotalAcb: deltas[i].PostStatus.TotalAcb}.X(),
			deltas[i].PostStatus,
		)
		AlmostEqual(t, gain, deltas[i].CapitalGain)
	}

	plo := ptf.LegacyOptions{}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, -5)

	// (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)

	/*
		buy 10
		sell 5 (superficial loss) -- min(5, 10, 1) / 5 * (loss of $5) = 1
		sell 4 (superficial loss) -- min(4, 10, 1) / 4 * (loss of $4.8) = 0.6
		wait
		sell 1 (loss, not superficial)
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell soon, causing superficial losses
	tx1 = makeTx(2, ptf.SELL, 5, 0.2)
	tx2 := makeTx(15, ptf.SELL, 4, 0.2)
	// Normal sell a while later
	tx3 := makeTx(100, ptf.SELL, 1, 0.2)
	txs = []*ptf.Tx{tx0, tx1, tx2, tx3}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	for i, _ := range deltas {
		fmt.Println(i, "Tx:", deltas[i].Tx, "PostStatus:", deltas[i].PostStatus)
	}
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, -4.0) // $1 superficial
	validate(2, 5, 7.0, 0.0)  // acb adjust
	validate(3, 1, 1.4, -3.6) // $1.2 superficial
	validate(4, 1, 2.6, 0.0)  // acb adjust
	validate(5, 0, 0.0, -2.4)

	/*
		buy 10
		wait
		sell 5 (superficial loss) -- min(5, 5, 10) / 5 = 1
		buy 5
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTx(50, ptf.SELL, 5, 0.2)
	tx2 = makeTx(51, ptf.BUY, 5, 0.2)
	txs = []*ptf.Tx{tx0, tx1, tx2}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 12.0, 0) // buy
	validate(1, 5, 6.0, 0)   // sell sfl $1
	validate(2, 5, 11.0, 0)  // sfl ACB adjust
	validate(3, 10, 14.0, 0) // buy

	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss) -- min(5, 5, 10) / 5 = 1
		buy 5 (in USD)
	*/
	tx0 = makeTxWithCurr(1, ptf.BUY, 10, 1.0, ptf.USD, 1.2)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTxWithCurr(50, ptf.SELL, 5, 0.2, ptf.USD, 1.2)
	tx2 = makeTxWithCurr(51, ptf.BUY, 5, 0.2, ptf.USD, 1.2)
	txs = []*ptf.Tx{tx0, tx1, tx2}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 14.4, 0) // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
	validate(1, 5, 7.2, 0)   // sell sfl $1 USD (1.2 CAD)
	validate(2, 5, 13.2, 0)  // sfl ACB adjust
	validate(3, 10, 16.8, 0) // buy

	/*
		buy 10
		wait
		sell 5 (loss)
		sell 5 (loss)
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTx(50, ptf.SELL, 5, 0.2)
	tx2 = makeTx(51, ptf.SELL, 5, 0.2)
	txs = []*ptf.Tx{tx0, tx1, tx2}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, -5.0)
	validate(2, 0, 0.0, -5.0)

	/*
		buy 100
		wait
		sell 99 (superficial loss) -- min(99, 25, 26) / 99 = 0.252525253
		buy 25
	*/
	tx0 = makeTx(1, ptf.BUY, 100, 3.0)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTx(50, ptf.SELL, 99, 2.0)
	tx2 = makeTx(51, ptf.BUY, 25, 2.2)
	txs = []*ptf.Tx{tx0, tx1, tx2}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 100, 302.0, 0)
	validate(1, 1, 3.02, -75.479999952) // total loss of 100.98, 25.500000048 is superficial
	validate(2, 1, 28.520000048, 0.0)   // acb adjust
	validate(3, 26, 85.520000048, 0)

	/*
		buy 10
		sell 10 (superficial loss) -- min(10, 15, 3) / 10 = 0.3
		buy 5
		sell 2 (superficial loss) -- min(2, 15, 3) / 2 = 1
		wait
		sell 3 (loss)
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell all
	tx1 = makeTx(2, ptf.SELL, 10, 0.2)
	tx2 = makeTx(3, ptf.BUY, 5, 1.0)
	tx3 = makeTx(4, ptf.SELL, 2, 0.2)
	tx4 := makeTx(50, ptf.SELL, 3, 0.2)
	txs = []*ptf.Tx{tx0, tx1, tx2, tx3, tx4}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 0, 0, -7)  // Superficial loss of 3
	validate(2, 0, 3, 0.0) // acb adjust
	validate(3, 5, 10.0, 0)
	validate(4, 3, 6.0, 0.0) // Superficial loss of 3.6
	validate(5, 3, 9.6, 0.0) // acb adjust
	validate(6, 0, 0, -9)

	/*
		buy 10
		sell 5 (gain)
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell causing gain
	tx1 = makeTx(2, ptf.SELL, 5, 2)
	txs = []*ptf.Tx{tx0, tx1}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, 4.0)

	// ************** Explicit Superficial Losses ***************************
	// Accurately specify a detected SFL
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss)
		buy 5 (in USD)
	*/
	tx0 = makeTxWithCurr(1, ptf.BUY, 10, 1.0, ptf.USD, 1.2)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTxV(50, ptf.SELL, 5, 0.2, ptf.USD, 1.2,
		util.NewOptional[ptf.SFLInput](ptf.SFLInput{-6.0, false}) /*SFL override in CAD*/)
	// ACB adjust is partial, as if splitting some to another affiliate.
	tx2 = makeTxV(50, ptf.SFLA, 5, 0.02, ptf.DEFAULT_CURRENCY, 1.0, util.Optional[ptf.SFLInput]{})
	tx3 = makeTxWithCurr(51, ptf.BUY, 5, 0.2, ptf.USD, 1.2)
	txs = []*ptf.Tx{tx0, tx1, tx2, tx3}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)

	rq.Nil(err)
	validate(0, 10, 14.4, 0) // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
	validate(1, 5, 7.2, 0.0) // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
	validate(2, 5, 7.3, 0)   // sfl ACB adjust 0.02 * 5
	validate(3, 10, 10.9, 0) // buy

	// Override a detected SFL
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss)
		buy 5 (in USD)
	*/
	tx0 = makeTxWithCurr(1, ptf.BUY, 10, 1.0, ptf.USD, 1.2)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTxV(50, ptf.SELL, 5, 0.2, ptf.USD, 1.2,
		util.NewOptional[ptf.SFLInput](ptf.SFLInput{-0.7, true}) /*SFL override in CAD*/)
	// ACB adjust is partial, as if splitting some to another affiliate.
	tx2 = makeTxV(50, ptf.SFLA, 5, 0.02, ptf.DEFAULT_CURRENCY, 1.0, util.Optional[ptf.SFLInput]{})
	tx3 = makeTxWithCurr(51, ptf.BUY, 5, 0.2, ptf.USD, 1.2)
	txs = []*ptf.Tx{tx0, tx1, tx2, tx3}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)

	rq.Nil(err)
	validate(0, 10, 14.4, 0)  // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
	validate(1, 5, 7.2, -5.3) // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
	validate(2, 5, 7.3, 0)    // sfl ACB adjust 0.02 * 5
	validate(3, 10, 10.9, 0)  // buy

	// Un-force the override, and check that we emit an error
	// Expect an error since we did not force.
	tx1.SpecifiedSuperficialLoss = util.NewOptional[ptf.SFLInput](ptf.SFLInput{-0.7, false})
	_, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.NotNil(err)

	// Add an un-detectable SFL (ie, the buy occurred in an untracked affiliate)
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (loss)
	*/
	tx0 = makeTxWithCurr(1, ptf.BUY, 10, 1.0, ptf.USD, 1.2)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTxV(50, ptf.SELL, 5, 0.2, ptf.USD, 1.2,
		util.NewOptional[ptf.SFLInput](ptf.SFLInput{-0.7, true}) /*SFL override in CAD*/)
	// ACB adjust is partial, as if splitting some to another affiliate.
	tx2 = makeTxV(50, ptf.SFLA, 5, 0.02, ptf.CAD, 1.0, util.Optional[ptf.SFLInput]{})
	txs = []*ptf.Tx{tx0, tx1, tx2}

	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 10, 14.4, 0)  // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
	validate(1, 5, 7.2, -5.3) // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
	validate(2, 5, 7.3, 0)    // sfl ACB adjust 0.02 * 5

	// Un-force the override, and check that we emit an error
	// Expect an error since we did not force.
	tx1.SpecifiedSuperficialLoss = util.NewOptional[ptf.SFLInput](ptf.SFLInput{-0.7, false})
	_, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.NotNil(err)

	// Currency errors
	// Sanity check for ok by itself.
	tx0 = makeTxV(50, ptf.SFLA, 1, 0.1, ptf.CAD, 1.0, util.Optional[ptf.SFLInput]{})
	txs = []*ptf.Tx{tx0}
	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 0, 0.1, 0)
	tx0 = makeTxV(50, ptf.SFLA, 1, 0.1, ptf.DEFAULT_CURRENCY, 1.0, util.Optional[ptf.SFLInput]{})
	txs = []*ptf.Tx{tx0}
	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	validate(0, 0, 0.1, 0)
	// Non 1.0 exchange rate
	tx0 = makeTxV(50, ptf.SFLA, 5, 0.02, ptf.USD, 1.0, util.Optional[ptf.SFLInput]{})
	txs = []*ptf.Tx{tx0}
	_, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.NotNil(err)
	// Non 1.0 exchange rate
	tx0 = makeTxV(50, ptf.SFLA, 5, 0.02, ptf.CAD, 1.1, util.Optional[ptf.SFLInput]{})
	txs = []*ptf.Tx{tx0}
	_, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.NotNil(err)
}

func TestBasicRocAcbErrors(t *testing.T) {
	// Test that RoC Txs always have zero shares
	sptf := SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 20.0}.X()
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	AddTxWithErr(t, tx, sptf)

	// Test that RoC cannot exceed the current ACB
	sptf = SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 20.0}.X()
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 13.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	AddTxWithErr(t, tx, sptf)

	// Test that RoC cannot occur on registered affiliates, since they have no ACB
	sptf = TPSS{Shares: 5, TotalAcb: NaN}.X()
	tx = TTx{Act: ptf.ROC, Shares: 0, Price: 3.0, AffName: "(R)"}.X()
	AddTxWithErr(t, tx, sptf)
}

func TestBasicRocAcb(t *testing.T) {
	rq := require.New(t)

	// Test basic ROC with different AllAffiliatesShareBalance
	sptf := &ptf.PortfolioSecurityStatus{
		Security: "FOO", ShareBalance: 2, AllAffiliatesShareBalance: 8, TotalAcb: 20.0,
	}
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 1.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{
			Security: "FOO", ShareBalance: 2, AllAffiliatesShareBalance: 8, TotalAcb: 18.0,
		},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test RoC with exchange
	sptf = SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 20.0}.X()
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 1.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		SimplePtfSecSt{Security: "FOO", Shares: 2, TotalAcb: 16.0}.X(),
	)
	rq.Equal(delta.CapitalGain, 0.0)
}

func TestRegisteredAffiliateCapitalGain(t *testing.T) {
	// Test there are no capital gains in registered accounts
	sptf := TPSS{Shares: 5, TotalAcb: NaN}.X()
	tx := TTx{Act: ptf.SELL, Shares: 2, Price: 3.0, AffName: "(R)"}.X()
	delta := AddTxNoErr(t, tx, sptf)
	StEq(t, TPSS{Shares: 3, AcbPerSh: NaN}.X(), delta.PostStatus)
	RqNaN(t, delta.CapitalGain)

	// Test that we fail if registered account sees non-nan acb
	sptf = TPSS{Shares: 5, TotalAcb: 0.0}.X()
	tx = TTx{Act: ptf.SELL, Shares: 2, Price: 3.0, AffName: "(R)"}.X()
	AddTxWithErr(t, tx, sptf)
	// Same, but non-zero acb
	sptf = TPSS{Shares: 5, TotalAcb: 1.0}.X()
	tx = TTx{Act: ptf.SELL, Shares: 2, Price: 3.0, AffName: "(R)"}.X()
	AddTxWithErr(t, tx, sptf)
	// Test that non-registered with NaN ACB generates an error as well
	sptf = TPSS{Shares: 5, TotalAcb: NaN}.X()
	tx = TTx{Act: ptf.SELL, Shares: 2, Price: 3.0}.X()
	AddTxWithErr(t, tx, sptf)
}

/*
Test cases TODO

- SFL with all sells on different affiliate

negative tests:
- SfLA Tx on registered affiliate

Functionality TODO
- SFL with buys split across affiliates
- Summary TX generation for multiple affiliates

*/

func TestAllAffiliateShareBalanceAddTx(t *testing.T) {
	rq := require.New(t)

	var sptf *ptf.PortfolioSecurityStatus
	var tx *ptf.Tx
	var delta *ptf.TxDelta

	// Basic buy
	sptf = TPSS{Shares: 3, AllShares: 7, TotalAcb: 15.0}.X()
	tx = TTx{Act: ptf.BUY, Shares: 2, Price: 5.0}.X()
	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(
		delta.PostStatus,
		TPSS{Shares: 5, AllShares: 9, TotalAcb: 25.0}.X())

	// Basic sell
	sptf = TPSS{Shares: 5, AllShares: 8, AcbPerSh: 3.0}.X()
	tx = TTx{Act: ptf.SELL, Shares: 2, Price: 5.0}.X()
	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(
		delta.PostStatus,
		TPSS{Shares: 3, AllShares: 6, AcbPerSh: 3.0}.X())

	// AllAffiliatesShareBalance too small (error).
	// In theory this could maybe panic, since it should not be possible, but
	// safer and easier to debug if we get a nicer error, which is in the API anyway.
	sptf = TPSS{Shares: 5, AllShares: 2, TotalAcb: 15.0}.X()
	tx = TTx{Act: ptf.SELL, Shares: 2, Price: 5.0}.X()
	AddTxWithErr(t, tx, sptf)
}

// Test Delta
type TDt struct {
	PostSt TPSS
	Gain   float64
}

func ValidateDeltas(t *testing.T, deltas []*ptf.TxDelta, expDeltas []TDt) {
	require.Equal(t, len(expDeltas), len(deltas))
	for i, delta := range deltas {
		fail := false
		fail = !SoftStEq(t, expDeltas[i].PostSt.X(), delta.PostStatus) || fail
		fail = !SoftAlmostEqual(t, expDeltas[i].Gain, deltas[i].CapitalGain) || fail
		if fail {
			require.FailNowf(t, "ValidateDeltas failed", "Delta %d", i)
		}
	}
}

func TestMultiAffiliateGains(t *testing.T) {
	rq := require.New(t)
	plo := ptf.LegacyOptions{}
	var txs []*ptf.Tx
	var deltas []*ptf.TxDelta
	var err error

	/*
		Default				Default (R)			B					B (R)
		--------				------------		---------		------------
		buy 10            buy 20            buy 30			buy 40
		sell 1 (gain)
								sell 2 ("gain")
														sell 3 (gain)
																			sell 4 ("gain")
	*/
	txs = []*ptf.Tx{
		// Buys
		TTx{Act: ptf.BUY, Shares: 10, Price: 1.0, AffName: ""}.X(),
		TTx{Act: ptf.BUY, Shares: 20, Price: 1.0, AffName: "(R)"}.X(),
		TTx{Act: ptf.BUY, Shares: 30, Price: 1.0, AffName: "B"}.X(),
		TTx{Act: ptf.BUY, Shares: 40, Price: 1.0, AffName: "B (R)"}.X(),
		// Sells
		TTx{Act: ptf.SELL, Shares: 1, Price: 1.2, AffName: ""}.X(),
		TTx{Act: ptf.SELL, Shares: 2, Price: 1.3, AffName: "(R)"}.X(),
		TTx{Act: ptf.SELL, Shares: 3, Price: 1.4, AffName: "B"}.X(),
		TTx{Act: ptf.SELL, Shares: 4, Price: 1.5, AffName: "B (R)"}.X(),
	}
	deltas, err = ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	ValidateDeltas(t, deltas, []TDt{
		// Buys
		TDt{PostSt: TPSS{Shares: 10, AllShares: 10, AcbPerSh: 1.0}},
		TDt{PostSt: TPSS{Shares: 20, AllShares: 30, TotalAcb: NaN}, Gain: NaN},
		TDt{PostSt: TPSS{Shares: 30, AllShares: 60, AcbPerSh: 1.0}},
		TDt{PostSt: TPSS{Shares: 40, AllShares: 100, TotalAcb: NaN}, Gain: NaN},
		// Sells
		TDt{PostSt: TPSS{Shares: 9, AllShares: 99, AcbPerSh: 1.0}, Gain: 1 * 0.2},
		TDt{PostSt: TPSS{Shares: 18, AllShares: 97, TotalAcb: NaN}, Gain: NaN},
		TDt{PostSt: TPSS{Shares: 27, AllShares: 94, AcbPerSh: 1.0}, Gain: 3 * 0.4},
		TDt{PostSt: TPSS{Shares: 36, AllShares: 90, TotalAcb: NaN}, Gain: NaN},
	})
}

func TestMultiAffiliateRoC(t *testing.T) {
	rq := require.New(t)
	plo := ptf.LegacyOptions{}

	/*
		Default				B
		--------				------------
		buy 10            buy 20
								ROC
		sell 10				sell 20
	*/
	txs := []*ptf.Tx{
		// Buys
		TTx{Act: ptf.BUY, Shares: 10, Price: 1.0, AffName: ""}.X(),
		TTx{Act: ptf.BUY, Shares: 20, Price: 1.0, AffName: "B"}.X(),
		// ROC
		TTx{Act: ptf.ROC, Shares: 0, Price: 0.2, AffName: "B"}.X(),
		// Sells
		TTx{Act: ptf.SELL, Shares: 10, Price: 1.1, AffName: ""}.X(),
		TTx{Act: ptf.SELL, Shares: 20, Price: 1.1, AffName: "B"}.X(),
	}
	deltas, err := ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	ValidateDeltas(t, deltas, []TDt{
		// Buys
		TDt{PostSt: TPSS{Shares: 10, AllShares: 10, AcbPerSh: 1.0}}, // Default
		TDt{PostSt: TPSS{Shares: 20, AllShares: 30, AcbPerSh: 1.0}}, // B
		// ROC
		TDt{PostSt: TPSS{Shares: 20, AllShares: 30, AcbPerSh: 0.8}}, // B
		// Sells
		TDt{PostSt: TPSS{Shares: 0, AllShares: 20, AcbPerSh: 0.0}, Gain: 10 * 0.1}, // Default
		TDt{PostSt: TPSS{Shares: 0, AllShares: 0, AcbPerSh: 0.0}, Gain: 20 * 0.3},  // B
	})
}

// TODO re-enable
func _TestOtherAffiliateSFL(t *testing.T) {
	rq := require.New(t)
	plo := ptf.LegacyOptions{}

	/*
		SFL with all sells on different affiliate

		Default				B
		--------				------------
		buy 10				buy 5
		wait...
		sell 2 (SFL)
								buy 2
	*/
	txs := []*ptf.Tx{
		TTx{TDate: mkDate(1), Act: ptf.BUY, Shares: 10, Price: 1.0, AffName: ""}.X(),
		TTx{TDate: mkDate(1), Act: ptf.BUY, Shares: 5, Price: 1.0, AffName: "B"}.X(),
		TTx{TDate: mkDate(40), Act: ptf.SELL, Shares: 2, Price: 0.5, AffName: ""}.X(),
		TTx{TDate: mkDate(41), Act: ptf.BUY, Shares: 2, Price: 1.0, AffName: "B"}.X(),
	}
	deltas, err := ptf.TxsToDeltaList(txs, nil, plo)
	rq.Nil(err)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: 10, AllShares: 10, TotalAcb: 10.0}}, // Buy in Default
		TDt{PostSt: TPSS{Shares: 5, AllShares: 15, TotalAcb: 5.0}},   // Buy in B
		TDt{PostSt: TPSS{Shares: 8, AllShares: 13, TotalAcb: 10.0}},  // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: 5, AllShares: 13, TotalAcb: 6.0}},   // Auto-adjust on B
		TDt{PostSt: TPSS{Shares: 7, AllShares: 15, TotalAcb: 8.0}},   // B
	})
}

func TestTxSort(t *testing.T) {
	txs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO3", SettlementDate: mkDate(3), Action: ptf.BUY, ReadIndex: 1},
		&ptf.Tx{Security: "FOO1", SettlementDate: mkDate(1), Action: ptf.BUY, ReadIndex: 2},
	}

	expTxs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO1", SettlementDate: mkDate(1), Action: ptf.BUY, ReadIndex: 2},
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO3", SettlementDate: mkDate(3), Action: ptf.BUY, ReadIndex: 1},
	}

	ptf.SortTxs(txs)
	require.Equal(t, txs, expTxs)
}
