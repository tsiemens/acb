package test

import (
	"testing"

	"github.com/shopspring/decimal"
	"github.com/stretchr/testify/require"

	decimal_opt "github.com/tsiemens/acb/decimal_value"
	ptf "github.com/tsiemens/acb/portfolio"
)

func AddTxNoErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) *ptf.TxDelta {
	delta, newTxs, err := addTx(tx, preTxStatus)
	require.Nil(t, newTxs)
	require.Nil(t, err)
	return delta
}

func AddTxWithErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) error {
	delta, newTxs, err := addTx(tx, preTxStatus)
	require.NotNil(t, err)
	require.Nil(t, delta)
	require.Nil(t, newTxs)
	return err
}

func TestBasicBuyAcb(t *testing.T) {
	var sptf *ptf.PortfolioSecurityStatus
	var tx *ptf.Tx
	var delta *ptf.TxDelta

	// Basic Buy
	sptf = ptf.NewEmptyPortfolioSecurityStatus(DefaultTestSecurity)
	tx = TTx{Act: ptf.BUY, Shares: DInt(3), Price: DFlt(10.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(3), TotalAcb: DOFlt(30.0)}, Gain: decimal_opt.Zero})

	// Test with commission
	tx = TTx{Act: ptf.BUY, Shares: DInt(2), Price: DFlt(10.0), Comm: DFlt(1.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(2), TotalAcb: DOFlt(21.0)}, Gain: decimal_opt.Zero})

	// Test with exchange rates
	sptf = TPSS{Shares: DInt(2), TotalAcb: DOFlt(21.0)}.X()
	tx = TTx{Act: ptf.BUY, Shares: DInt(3), Price: DFlt(12.0), Comm: DFlt(1.0),
		Curr: ptf.USD, FxRate: DFlt(2.0),
		CommCurr: "XXX", CommFxRate: DFlt(0.3)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(5),
			TotalAcb: DOFlt(21.0).AddD(DFlt(2 * 36.0)).AddD(DFlt(0.3))},
			Gain: decimal_opt.Zero})
}

func TestBasicSellAcbErrors(t *testing.T) {
	// Sell more shares than available
	sptf := TPSS{Shares: DInt(2), TotalAcb: DOFlt(20.0)}.X()
	tx := TTx{Act: ptf.SELL, Shares: DInt(3), Price: DFlt(10.0)}.X()
	AddTxWithErr(t, tx, sptf)
}

func TestBasicSellAcb(t *testing.T) {
	// Sell all remaining shares
	sptf := TPSS{Shares: DInt(2), TotalAcb: DOFlt(20.0)}.X()
	tx := TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(15.0)}.X()

	delta := AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal_opt.Zero}, Gain: DOFlt(10.0)})

	// Sell shares with commission
	sptf = TPSS{Shares: DInt(3), TotalAcb: DOFlt(30.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(15.0), Comm: DFlt(1.0)}.X()

	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(1), TotalAcb: DOFlt(10.0)}, Gain: DOFlt(9.0)})

	// Sell shares with exchange rate
	sptf = TPSS{Shares: DInt(3), TotalAcb: DOFlt(30.0)}.X()
	tx = TTx{
		Act: ptf.SELL, Shares: DInt(2), Price: DFlt(15.0), Comm: DFlt(2.0),
		Curr: "XXX", FxRate: DFlt(2.0),
		CommCurr: "YYY", CommFxRate: DFlt(0.4)}.X()

	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(1), TotalAcb: DOFlt(10.0)}, Gain: DOFlt((15.0 * 2.0 * 2.0) - 20.0 - 0.8)})
}

func TxsToDeltaListNoErr(t *testing.T, txs []*ptf.Tx) []*ptf.TxDelta {
	deltas, err := ptf.TxsToDeltaList(txs, nil, ptf.LegacyOptions{})
	require.Nil(t, err)
	return deltas
}

func TxsToDeltaListWithErr(t *testing.T, txs []*ptf.Tx) error {
	_, err := ptf.TxsToDeltaList(txs, nil, ptf.LegacyOptions{})
	require.NotNil(t, err)
	return err
}

func TestSuperficialLosses(t *testing.T) {
	var deltas []*ptf.TxDelta

	/*
		buy 10
		wait
		sell 5 (loss, not superficial)
	*/
	txs := []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell half at a loss a while later, for a total of $1
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(12.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(6.0)}, Gain: DOFlt(-5.0)},
	})

	// (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)

	/*
		buy 10
		sell 5 (superficial loss) -- min(5, 10, 1) / 5 * (loss of $5) = 1
		sell 4 (superficial loss) -- min(4, 10, 1) / 4 * (loss of $4.8) = 0.6
		wait
		sell 1 (loss, not superficial)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell soon, causing superficial losses
		TTx{TDay: 2, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(),
		TTx{TDay: 15, Act: ptf.SELL, Shares: DFlt(4), Price: DFlt(0.2)}.X(),
		// Normal sell a while later
		TTx{TDay: 100, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(12.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(6.0)}, Gain: DOFlt(-4.0)},      // $1 superficial
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.0)}, Gain: decimal_opt.Zero}, // acb adjust
		{PostSt: TPSS{Shares: DInt(1), TotalAcb: DOFlt(1.4)}, Gain: DOFlt(-3.6)},      // $1.2 superficial
		{PostSt: TPSS{Shares: DInt(1), TotalAcb: DOFlt(2.6)}, Gain: decimal_opt.Zero}, // acb adjust
		{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal_opt.Zero}, Gain: DOFlt(-2.4)},
	})

	/*
		buy 10
		wait
		sell 5 (superficial loss) -- min(5, 5, 10) / 5 = 1
		buy 5
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(0.2), Comm: DFlt(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(12.0)}, Gain: decimal_opt.Zero}, // buy
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(6.0)}, Gain: decimal_opt.Zero},   // sell sfl $1
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(11.0)}, Gain: decimal_opt.Zero},  // sfl ACB adjust
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(14.0)}, Gain: decimal_opt.Zero}, // buy
	})

	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss) -- min(5, 5, 10) / 5 = 1
		buy 5 (in USD)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(14.4)}, Gain: decimal_opt.Zero}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.2)}, Gain: decimal_opt.Zero},   // sell sfl $1 USD (1.2 CAD)
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(13.2)}, Gain: decimal_opt.Zero},  // sfl ACB adjust
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(16.8)}, Gain: decimal_opt.Zero}, // buy
	})

	/*
		buy 10
		wait
		sell 5 (loss)
		sell 5 (loss)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(),
		TTx{TDay: 51, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(12.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(6.0)}, Gain: DOFlt(-5.0)},
		{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal_opt.Zero}, Gain: DOFlt(-5.0)},
	})

	/*
		buy 100
		wait
		sell 99 (superficial loss) -- min(99, 25, 26) / 99 = 0.252525253
		buy 25
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DFlt(100), Price: DFlt(3.0), Comm: DFlt(2.0)}.X(), // Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DFlt(99), Price: DFlt(2.0)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: DFlt(25), Price: DFlt(2.2), Comm: DFlt(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DFlt(100), TotalAcb: DOFlt(302.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DInt(1), TotalAcb: DOFlt(3.02)}, Gain: DOStr("-75.48000000000000255")},     // total loss of 100.98, 25.500000048 is superficial
		{PostSt: TPSS{Shares: DInt(1), TotalAcb: DOStr("28.51999999999999745")}, Gain: decimal_opt.Zero}, // acb adjust
		{PostSt: TPSS{Shares: DFlt(26), TotalAcb: DOStr("85.51999999999999745")}, Gain: decimal_opt.Zero},
	})

	/*
		buy 10
		sell 10 (superficial loss) -- min(10, 15, 3) / 10 = 0.3
		buy 5
		sell 2 (superficial loss) -- min(2, 15, 3) / 2 = 1
		wait
		sell 3 (loss)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell all
		TTx{TDay: 2, Act: ptf.SELL, Shares: DInt(10), Price: DFlt(0.2)}.X(),
		TTx{TDay: 3, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		TTx{TDay: 4, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(3), Price: DFlt(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(12.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DFlt(0), TotalAcb: DOFlt(0)}, Gain: DOFlt(-7)},        // Superficial loss of 3
		{PostSt: TPSS{Shares: DFlt(0), TotalAcb: DOFlt(3)}, Gain: decimal_opt.Zero}, // acb adjust
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(10.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DInt(3), TotalAcb: DOFlt(6.0)}, Gain: decimal_opt.Zero}, // Superficial loss of 3.6
		{PostSt: TPSS{Shares: DInt(3), TotalAcb: DOFlt(9.6)}, Gain: decimal_opt.Zero}, // acb adjust
		{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal_opt.Zero}, Gain: DOFlt(-9)},
	})

	/*
		buy 10
		sell 5 (gain)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell causing gain
		TTx{TDay: 2, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(12.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(6.0)}, Gain: DOFlt(4.0)},
	})

	/* Fractional shares SFL avoidance
	   With floats, this would be hard, because we wouldn't come exactly back to zero.
	   We get around this by using Decimal

	   buy 5.0
	   sell 4.7
	   sell 0.3 (loss) (not superficial because we sold all shares and should have zero)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DFlt(5.0), Price: DFlt(1.0), Comm: DFlt(2.0)}.X(),
		// Sell all in two fractional operations
		TTx{TDay: 2, Act: ptf.SELL, Shares: DStr("4.7"), Price: DFlt(0.2)}.X(),
		TTx{TDay: 3, Act: ptf.SELL, Shares: DStr("0.3"), Price: DFlt(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.0)}, Gain: decimal_opt.Zero},
		{PostSt: TPSS{Shares: DStr("0.3"), TotalAcb: DOFlt(0.42)}, Gain: DOFlt(-5.64)},
		{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal_opt.Zero}, Gain: DOFlt(-0.36)},
	})

	// ************** Explicit Superficial Losses ***************************
	// Accurately specify a detected SFL
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss)
		buy 5 (in USD)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2), SFL: CADSFL(DFlt(-6.0), false)}.X(),
		// ACB adjust is partial, as if splitting some to another affiliate.
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(5), Price: DFlt(0.02), Curr: EXP_DEFAULT_CURRENCY, FxRate: DFlt(1.0)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(14.4)}, Gain: decimal_opt.Zero}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.2)}, Gain: decimal_opt.Zero},   // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.3)}, Gain: decimal_opt.Zero},   // sfl ACB adjust 0.02 * 5
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(10.9)}, Gain: decimal_opt.Zero}, // buy
	})

	// Override a detected SFL
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss)
		buy 5 (in USD)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2), SFL: CADSFL(DFlt(-0.7), true)}.X(),
		// ACB adjust is partial, as if splitting some to another affiliate.
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(5), Price: DFlt(0.02), Curr: EXP_DEFAULT_CURRENCY, FxRate: DFlt(1.0)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(14.4)}, Gain: decimal_opt.Zero}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.2)}, Gain: DOFlt(-5.3)},        // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.3)}, Gain: decimal_opt.Zero},   // sfl ACB adjust 0.02 * 5
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(10.9)}, Gain: decimal_opt.Zero}, // buy
	})

	// Un-force the override, and check that we emit an error
	// Expect an error since we did not force.
	txs[1].SpecifiedSuperficialLoss = CADSFL(DFlt(-0.7), false)
	TxsToDeltaListWithErr(t, txs)

	// Add an un-detectable SFL (ie, the buy occurred in an untracked affiliate)
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (loss)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), Curr: ptf.USD, FxRate: DFlt(1.2), Comm: DFlt(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2), Curr: ptf.USD, FxRate: DFlt(1.2), SFL: CADSFL(DFlt(-0.7), true)}.X(),
		// ACB adjust is partial, as if splitting some to another affiliate.
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(5), Price: DFlt(0.02), Curr: ptf.CAD, FxRate: DFlt(1.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), TotalAcb: DOFlt(14.4)}, Gain: decimal_opt.Zero}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.2)}, Gain: DOFlt(-5.3)},        // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
		{PostSt: TPSS{Shares: DInt(5), TotalAcb: DOFlt(7.3)}, Gain: decimal_opt.Zero},   // sfl ACB adjust 0.02 * 5
	})

	// Un-force the override, and check that we emit an error
	// Expect an error since we did not force.
	txs[1].SpecifiedSuperficialLoss = CADSFL(DFlt(-0.7), false)
	TxsToDeltaListWithErr(t, txs)

	// Currency errors
	// Sanity check for ok by itself.
	txs = []*ptf.Tx{
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(1), Price: DFlt(0.1), Curr: ptf.CAD, FxRate: DFlt(1.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: DOFlt(0.1)}, Gain: decimal_opt.Zero},
	})

	txs = []*ptf.Tx{
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(1), Price: DFlt(0.1), Curr: EXP_DEFAULT_CURRENCY, FxRate: DFlt(1.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: DOFlt(0.1)}, Gain: decimal_opt.Zero},
	})
	// Non 1.0 exchange rate
	txs = []*ptf.Tx{
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(5), Price: DFlt(0.02), Curr: ptf.USD, FxRate: DFlt(1.0)}.X(),
	}
	TxsToDeltaListWithErr(t, txs)
	txs = []*ptf.Tx{
		// Non 1.0 exchange rate
		TTx{TDay: 50, Act: ptf.SFLA, Shares: DInt(5), Price: DFlt(0.02), Curr: ptf.CAD, FxRate: DFlt(1.1)}.X(),
	}
	TxsToDeltaListWithErr(t, txs)
}

func TestBasicRocAcbErrors(t *testing.T) {
	// Test that RoC Txs always have zero shares
	sptf := TPSS{Shares: DInt(2), TotalAcb: DOFlt(20.0)}.X()
	tx := TTx{Act: ptf.ROC, Shares: DInt(3), Price: DFlt(10.0)}.X()
	AddTxWithErr(t, tx, sptf)

	// Test that RoC cannot exceed the current ACB
	sptf = TPSS{Shares: DInt(2), TotalAcb: DOFlt(20.0)}.X()
	tx = TTx{Act: ptf.ROC, Price: DFlt(13.0)}.X()
	AddTxWithErr(t, tx, sptf)

	// Test that RoC cannot occur on registered affiliates, since they have no ACB
	sptf = TPSS{Shares: DInt(5), TotalAcb: decimal_opt.Null}.X()
	tx = TTx{Act: ptf.ROC, Shares: decimal.Zero, Price: DFlt(3.0), AffName: "(R)"}.X()
	AddTxWithErr(t, tx, sptf)
}

func TestBasicRocAcb(t *testing.T) {
	// Test basic ROC with different AllAffiliatesShareBalance
	sptf := TPSS{Shares: DInt(2), AllShares: DInt(8), TotalAcb: DOFlt(20.0)}.X()
	tx := TTx{Act: ptf.ROC, Price: DFlt(1.0)}.X()

	delta := AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(2), AllShares: DInt(8), TotalAcb: DOFlt(18.0)}, Gain: decimal_opt.Zero})

	// Test RoC with exchange
	sptf = TPSS{Shares: DInt(2), TotalAcb: DOFlt(20.0)}.X()
	tx = TTx{Act: ptf.ROC, Price: DFlt(1.0), FxRate: DFlt(2.0)}.X()

	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(2), TotalAcb: DOFlt(16.0)}, Gain: decimal_opt.Zero})
}

func TestBasicSflaErrors(t *testing.T) {
	rq := require.New(t)
	// Test than an SfLA on a registered affiliate is invalid
	sptf := TPSS{Shares: DInt(2), TotalAcb: decimal_opt.Null}.X()
	tx := TTx{Act: ptf.SFLA, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X()
	err := AddTxWithErr(t, tx, sptf)
	rq.Regexp("Registered affiliates do not have an ACB", err)
}

func TestRegisteredAffiliateCapitalGain(t *testing.T) {
	crq := NewCustomRequire(t)
	// Test there are no capital gains in registered accounts
	sptf := TPSS{Shares: DInt(5), TotalAcb: decimal_opt.Null}.X()
	tx := TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(3.0), AffName: "(R)"}.X()
	delta := AddTxNoErr(t, tx, sptf)
	crq.Equal(TPSS{Shares: DInt(3), AcbPerSh: decimal_opt.Null}.X(), delta.PostStatus)
	//RqNaN(t, delta.CapitalGain) TODO

	// Test that we fail if registered account sees non-nan acb
	sptf = TPSS{Shares: DInt(5), TotalAcb: decimal_opt.Zero}.X()
	tx = TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(3.0), AffName: "(R)"}.X()
	RqPanicsWithRegexp(t, "bad null optional value", func() {
		AddTxWithErr(t, tx, sptf)
	})
	// Same, but non-zero acb
	sptf = TPSS{Shares: DInt(5), TotalAcb: DOFlt(1.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(3.0), AffName: "(R)"}.X()
	RqPanicsWithRegexp(t, "bad null optional value", func() {
		AddTxWithErr(t, tx, sptf)
	})
	// Test that non-registered with NaN ACB generates an error as well
	sptf = TPSS{Shares: DInt(5), TotalAcb: decimal_opt.Null}.X()
	tx = TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(3.0)}.X()
	RqPanicsWithRegexp(t, "bad null optional value", func() {
		AddTxWithErr(t, tx, sptf)
	})
}

func TestAllAffiliateShareBalanceAddTx(t *testing.T) {
	var sptf *ptf.PortfolioSecurityStatus
	var tx *ptf.Tx
	var delta *ptf.TxDelta

	// Basic buy
	sptf = TPSS{Shares: DInt(3), AllShares: DInt(7), TotalAcb: DOFlt(15.0)}.X()
	tx = TTx{Act: ptf.BUY, Shares: DInt(2), Price: DFlt(5.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(9), TotalAcb: DOFlt(25.0)}})

	// Basic sell
	sptf = TPSS{Shares: DInt(5), AllShares: DInt(8), AcbPerSh: DOFlt(3.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(5.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: DInt(3), AllShares: DFlt(6.0), AcbPerSh: DOFlt(3.0)}, Gain: DOFlt(4.0)})

	// AllAffiliatesShareBalance too small (error).
	// In theory this could maybe panic, since it should not be possible, but
	// safer and easier to debug if we get a nicer error, which is in the API anyway.
	sptf = TPSS{Shares: DInt(5), AllShares: DInt(2), TotalAcb: DOFlt(15.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(5.0)}.X()
	AddTxWithErr(t, tx, sptf)
}

func TestMultiAffiliateGains(t *testing.T) {
	var txs []*ptf.Tx
	var deltas []*ptf.TxDelta

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
		TTx{Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{Act: ptf.BUY, Shares: DInt(20), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{Act: ptf.BUY, Shares: DInt(30), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{Act: ptf.BUY, Shares: DInt(40), Price: DFlt(1.0), AffName: "B (R)"}.X(),
		// Sells
		TTx{Act: ptf.SELL, Shares: DInt(1), Price: DFlt(1.2), AffName: ""}.X(),
		TTx{Act: ptf.SELL, Shares: DInt(2), Price: DFlt(1.3), AffName: "(R)"}.X(),
		TTx{Act: ptf.SELL, Shares: DInt(3), Price: DFlt(1.4), AffName: "B"}.X(), TTx{Act: ptf.SELL, Shares: DInt(4), Price: DFlt(1.5), AffName: "B (R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		// Buys
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), AcbPerSh: DOFlt(1.0)}},
		{PostSt: TPSS{Shares: DInt(20), AllShares: DInt(30), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null},
		{PostSt: TPSS{Shares: DInt(30), AllShares: DInt(60), AcbPerSh: DOFlt(1.0)}},
		{PostSt: TPSS{Shares: DInt(40), AllShares: DInt(100), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null},
		// Sells
		{PostSt: TPSS{Shares: DInt(9), AllShares: DInt(99), AcbPerSh: DOFlt(1.0)}, Gain: DOFlt(1 * 0.2)},
		{PostSt: TPSS{Shares: DInt(18), AllShares: DInt(97), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null},
		{PostSt: TPSS{Shares: DInt(27), AllShares: DInt(94), AcbPerSh: DOFlt(1.0)}, Gain: DOFlt(3 * 0.4)},
		{PostSt: TPSS{Shares: DInt(36), AllShares: DInt(90), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null},
	})
}

func TestMultiAffiliateRoC(t *testing.T) {
	/*
		Default				B
		--------				------------
		buy 10            buy 20
								ROC
		sell 10				sell 20
	*/
	txs := []*ptf.Tx{
		// Buys
		TTx{Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{Act: ptf.BUY, Shares: DInt(20), Price: DFlt(1.0), AffName: "B"}.X(),
		// ROC
		TTx{Act: ptf.ROC, Shares: decimal.Zero, Price: DFlt(0.2), AffName: "B"}.X(),
		// Sells
		TTx{Act: ptf.SELL, Shares: DInt(10), Price: DFlt(1.1), AffName: ""}.X(),
		TTx{Act: ptf.SELL, Shares: DInt(20), Price: DFlt(1.1), AffName: "B"}.X(),
	}
	deltas := TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		// Buys
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), AcbPerSh: DOFlt(1.0)}}, // Default
		{PostSt: TPSS{Shares: DInt(20), AllShares: DInt(30), AcbPerSh: DOFlt(1.0)}}, // B
		// ROC
		{PostSt: TPSS{Shares: DInt(20), AllShares: DInt(30), AcbPerSh: DOFlt(0.8)}}, // B
		// Sells
		{PostSt: TPSS{Shares: decimal.Zero, AllShares: DInt(20), AcbPerSh: decimal_opt.Zero}, Gain: DOFlt(10 * 0.1)},     // Default
		{PostSt: TPSS{Shares: decimal.Zero, AllShares: decimal.Zero, AcbPerSh: decimal_opt.Zero}, Gain: DOFlt(20 * 0.3)}, // B
	})
}

func TestOtherAffiliateSFL(t *testing.T) {
	/* SFL with all buys on different affiliate

	Default				B
	--------				------------
	buy 10				buy 5
	wait...
	sell 2 (SFL)
							buy 2
	*/
	txs := []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
	}
	deltas := TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                 // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), TotalAcb: DOFlt(5.0)}},                   // Buy in B
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(13), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)}, // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(13), TotalAcb: DOFlt(6.0)}},                   // Auto-adjust on B
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(15), TotalAcb: DOFlt(8.0)}},                   // B
	})

	/* SFL with all buys on registered affiliate
	   (same txs as above)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                             // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(13), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)},             // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(15), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
	})

	/* SFL with all buys on other affiliate B, but sells on a second affiliate (R)
	Make sure it doesn't interfere or cause errors.
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                             // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), TotalAcb: DOFlt(5.0)}},                               // Buy in B
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(20), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(18), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)},             // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(18), TotalAcb: DOFlt(6.0)}},                               // Auto-adjust on B
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(20), TotalAcb: DOFlt(8.0)}},                               // Buy in B
		{PostSt: TPSS{Shares: DInt(3), AllShares: DInt(18), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Sell in (R)
	})

	/* SFL with buys on two other affiliates (both non-registered)
	Default			B			C
	--------			------	-------
	buy 10			buy 5		buy 7
	wait...
	sell 2 (SFL)
						buy 2		buy 2
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.0), AffName: "C"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "C"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                 // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), TotalAcb: DOFlt(5.0)}},                   // Buy in B
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: DOFlt(7.0)}},                   // Buy in C
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(20), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)}, // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(20), TotalAcb: DOFlt(5.4375)}},                // Auto-adjust on B. Gets 7/16 (43.75%) of the SFL
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(20), TotalAcb: DOFlt(7.5625)}},                // Auto-adjust on C. Gets 9/16 (56.25%) of the SFL
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: DOFlt(7.4375)}},                // Buy in B
		{PostSt: TPSS{Shares: DInt(9), AllShares: DInt(24), TotalAcb: DOFlt(9.5625)}},                // Buy in C
	})

	/* SFL with buys on two other affiliates (registered/non-registered)
	Default			(R)		B
	--------			------	-------
	buy 10			buy 5		buy 7
	wait...
	sell 2 (SFL)
						buy 2		buy 2
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                             // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), AcbPerSh: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: DOFlt(7.0)}},                               // Buy in B
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(20), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)},             // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(20), TotalAcb: DOFlt(7.5625)}},                            // Auto-adjust on B. Gets 9/16 (56.25%) of the SFL
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(9), AllShares: DInt(24), TotalAcb: DOFlt(9.5625)}},                            // Buy in B
	})

	/* SFL with buys on one other affiliate, but fewer shares in the only selling
	affiliate than the shares affected by the superficial loss.

	Default			B
	--------			------------
	buy 5
	wait...
	sell 4 (SFL)
						buy 2
						sell 1
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(4), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 42, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.0), AffName: "B"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(5), TotalAcb: DOFlt(5.0)}}, // Buy in Default
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(1), TotalAcb: DOFlt(1.0)}, Gain: DOFlt(-1.0), SFL: DOFlt(-1.0),
			PotentiallyOverAppliedSfl: true}, // SFL of 0.5 * 2(/4) shares
		{PostSt: TPSS{Shares: decimal.Zero, AllShares: DInt(1), TotalAcb: DOFlt(1.0)}},               // auto adjust on B (100%)
		{PostSt: TPSS{Shares: DInt(2), AllShares: DInt(3), TotalAcb: DOFlt(3.0)}},                    // Buy in B
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(2), TotalAcb: DOFlt(1.5)}, Gain: DOFlt(0.50)}, // Sell in B
	})

	/* SFL with buys on both SFL affiliate and one other affiliate.

	Default			B
	--------			------------
	buy 5
	wait...
	sell 4 (SFL)
						buy 2
	buy 1
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(4), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 42, Act: ptf.BUY, Shares: DInt(1), Price: DFlt(2.0), AffName: ""}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(5), TotalAcb: DOFlt(5.0)}},                                      // Buy in Default
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(1), TotalAcb: DOFlt(1.0)}, Gain: DOFlt(-0.5), SFL: DOFlt(-1.5)}, // SFL of 0.5 * 3(/4) shares
		{PostSt: TPSS{Shares: decimal.Zero, AllShares: DInt(1), TotalAcb: DOFlt(0.75)}},                                // auto adjust on B (50%)
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(1), TotalAcb: DOFlt(1.75)}},                                     // auto adjust on default (50%)
		{PostSt: TPSS{Shares: DInt(2), AllShares: DInt(3), TotalAcb: DOFlt(2.75)}},                                     // Buy in B
		{PostSt: TPSS{Shares: DInt(2), AllShares: DInt(4), TotalAcb: DOFlt(3.75)}},                                     // Buy in default
	})

	/* SFL with buy on one other registered affiliate.

	Default			(R)
	--------			------------
	buy 5
	wait...
	sell 4 (SFL)
						buy 2
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(4), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(5), TotalAcb: DOFlt(5.0)}},                                      // Buy in Default
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(1), TotalAcb: DOFlt(1.0)}, Gain: DOFlt(-1.0), SFL: DOFlt(-1.0)}, // SFL of 0.5 * 2(/4) shares
		{PostSt: TPSS{Shares: DInt(2), AllShares: DInt(3), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null},        // Buy in B
	})

	/* SFL with buy on one other registered affiliate, but fewer shares in the only
	selling affiliate than the shares affected by the superficial loss.

	Default			(R)
	--------			------------
	buy 5
	wait...
	sell 4 (SFL)
						buy 2
						sell 1
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(4), Price: DFlt(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 42, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(5), TotalAcb: DOFlt(5.0)}}, // Buy in Default
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(1), TotalAcb: DOFlt(1.0)}, Gain: DOFlt(-1.0), SFL: DOFlt(-1.0),
			PotentiallyOverAppliedSfl: true}, // SFL of 0.5 * 2(/4) shares
		{PostSt: TPSS{Shares: DInt(2), AllShares: DInt(3), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(1), AllShares: DInt(2), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Sell in (R)
	})
}

func TestOtherAffiliateExplicitSFL(t *testing.T) {
	// rq := require.New(t)

	/* SFL with sells on two other affiliates (both non-registered),
	   and explicitly set the SFLA (dubiously) on one of the affiliates.
	Default			B			C
	--------			------	-------
	buy 10			buy 5		buy 7
	wait...
	sell 2 (explicit SFL)
	                        SFLA
						buy 2		buy 2
	*/
	txs := []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.0), AffName: "C"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: "", SFL: CADSFL(DFlt(-1.0), false)}.X(),
		TTx{TDay: 40, Act: ptf.SFLA, Shares: DInt(1), Price: DFlt(0.5), AffName: "C"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "C"}.X(),
	}
	deltas := TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                 // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), AcbPerSh: DOFlt(1.0)}},                   // Buy in B
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: DOFlt(7.0)}},                   // Buy in C
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(20), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)}, // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(20), TotalAcb: DOFlt(7.5)}},                   // Explicit adjust on C
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), AcbPerSh: DOFlt(1.0)}},                   // Buy in B
		{PostSt: TPSS{Shares: DInt(9), AllShares: DInt(24), TotalAcb: DOFlt(9.5)}},                   // Buy in C
	})

	/* SFL with sells on two other affiliates (registered/non-registered),
		with explicit SFL
	Default			(R)		B
	--------			------	-------
	buy 10			buy 5		buy 7
	wait...
	sell 2 (expicit SFL)
									SFLA
						buy 2		buy 2
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(10), Price: DFlt(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.0), AffName: "B"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.5), AffName: "", SFL: CADSFL(DFlt(-1.0), false)}.X(),
		TTx{TDay: 40, Act: ptf.SFLA, Shares: DInt(1), Price: DFlt(0.5), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(1.0), AffName: "B"}.X(),
	}

	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		{PostSt: TPSS{Shares: DInt(10), AllShares: DInt(10), TotalAcb: DOFlt(10.0)}},                             // Buy in Default
		{PostSt: TPSS{Shares: DInt(5), AllShares: DInt(15), AcbPerSh: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: DOFlt(7.0)}},                               // Buy in B
		{PostSt: TPSS{Shares: DInt(8), AllShares: DInt(20), TotalAcb: DOFlt(8.0)}, SFL: DOFlt(-1.0)},             // SFL of 0.5 * 2 shares
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(20), TotalAcb: DOFlt(7.5)}},                               // Explicit adjust on B
		{PostSt: TPSS{Shares: DInt(7), AllShares: DInt(22), TotalAcb: decimal_opt.Null}, Gain: decimal_opt.Null}, // Buy in (R)
		{PostSt: TPSS{Shares: DInt(9), AllShares: DInt(24), TotalAcb: DOFlt(9.5)}},                               // Buy in B
	})
}

func TestTxSort(t *testing.T) {
	txs := []*ptf.Tx{
		{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.SELL, ReadIndex: 0},
		{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.BUY, ReadIndex: 3},
		{Security: "FOO3", SettlementDate: mkDate(3), Action: ptf.BUY, ReadIndex: 1},
		{Security: "FOO1", SettlementDate: mkDate(1), Action: ptf.BUY, ReadIndex: 2},
	}

	expTxs := []*ptf.Tx{
		{Security: "FOO1", SettlementDate: mkDate(1), Action: ptf.BUY, ReadIndex: 2},
		{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.SELL, ReadIndex: 0},
		{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.BUY, ReadIndex: 3},
		{Security: "FOO3", SettlementDate: mkDate(3), Action: ptf.BUY, ReadIndex: 1},
	}

	ptf.SortTxs(txs)
	require.Equal(t, txs, expTxs)
}
