package test

import (
	"testing"

	"github.com/stretchr/testify/require"

	decimal "github.com/tsiemens/acb/decimal_value"
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
	tx = TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(3), Price: decimal.NewFromFloat(10.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(30.0)}, Gain: decimal.NewFromFloat(0.0)})

	// Test with commission
	tx = TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(10.0), Comm: decimal.NewFromFloat(1.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(21.0)}, Gain: decimal.NewFromFloat(0.0)})

	// Test with exchange rates
	sptf = TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(21.0)}.X()
	tx = TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(3), Price: decimal.NewFromFloat(12.0), Comm: decimal.NewFromFloat(1.0),
		Curr: ptf.USD, FxRate: decimal.NewFromFloat(2.0),
		CommCurr: "XXX", CommFxRate: decimal.NewFromFloat(0.3)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(21.0).Add(decimal.NewFromFloat(2 * 36.0)).Add(decimal.NewFromFloat(0.3))}, Gain: decimal.NewFromFloat(0.0)})
}

func TestBasicSellAcbErrors(t *testing.T) {
	// Sell more shares than available
	sptf := TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(20.0)}.X()
	tx := TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(3), Price: decimal.NewFromFloat(10.0)}.X()
	AddTxWithErr(t, tx, sptf)
}

func TestBasicSellAcb(t *testing.T) {
	// Sell all remaining shares
	sptf := TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(20.0)}.X()
	tx := TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(15.0)}.X()

	delta := AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal.Zero}, Gain: decimal.NewFromFloat(10.0)})

	// Sell shares with commission
	sptf = TPSS{Shares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(30.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(15.0), Comm: decimal.NewFromFloat(1.0)}.X()

	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(10.0)}, Gain: decimal.NewFromFloat(9.0)})

	// Sell shares with exchange rate
	sptf = TPSS{Shares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(30.0)}.X()
	tx = TTx{
		Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(15.0), Comm: decimal.NewFromFloat(2.0),
		Curr: "XXX", FxRate: decimal.NewFromFloat(2.0),
		CommCurr: "YYY", CommFxRate: decimal.NewFromFloat(0.4)}.X()

	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(10.0)}, Gain: decimal.NewFromFloat((15.0 * 2.0 * 2.0) - 20.0 - 0.8)})
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell half at a loss a while later, for a total of $1
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(12.0)}, Gain: decimal.NewFromFloat(0.0)},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(6.0)}, Gain: decimal.NewFromFloat(-5.0)},
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell soon, causing superficial losses
		TTx{TDay: 2, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2)}.X(),
		TTx{TDay: 15, Act: ptf.SELL, Shares: decimal.NewFromFloat(4), Price: decimal.NewFromFloat(0.2)}.X(),
		// Normal sell a while later
		TTx{TDay: 100, Act: ptf.SELL, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(12.0)}, Gain: decimal.Zero},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(6.0)}, Gain: decimal.NewFromFloat(-4.0)}, // $1 superficial
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.0)}, Gain: decimal.Zero},                    // acb adjust
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.4)}, Gain: decimal.NewFromFloat(-3.6)}, // $1.2 superficial
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(2.6)}, Gain: decimal.Zero},                    // acb adjust
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal.Zero}, Gain: decimal.NewFromFloat(-2.4)},
	})

	/*
		buy 10
		wait
		sell 5 (superficial loss) -- min(5, 5, 10) / 5 = 1
		buy 5
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Comm: decimal.NewFromFloat(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(12.0)}, Gain: decimal.Zero}, // buy
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(6.0)}, Gain: decimal.Zero},   // sell sfl $1
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(11.0)}, Gain: decimal.Zero},  // sfl ACB adjust
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(14.0)}, Gain: decimal.Zero}, // buy
	})

	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (superficial loss) -- min(5, 5, 10) / 5 = 1
		buy 5 (in USD)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(14.4)}, Gain: decimal.Zero}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.2)}, Gain: decimal.Zero},   // sell sfl $1 USD (1.2 CAD)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(13.2)}, Gain: decimal.Zero},  // sfl ACB adjust
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(16.8)}, Gain: decimal.Zero}, // buy
	})

	/*
		buy 10
		wait
		sell 5 (loss)
		sell 5 (loss)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2)}.X(),
		TTx{TDay: 51, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(12.0)}, Gain: decimal.Zero},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(6.0)}, Gain: decimal.NewFromFloat(-5.0)},
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal.Zero}, Gain: decimal.NewFromFloat(-5.0)},
	})

	/*
		buy 100
		wait
		sell 99 (superficial loss) -- min(99, 25, 26) / 99 = 0.252525253
		buy 25
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromFloat(100), Price: decimal.NewFromFloat(3.0), Comm: decimal.NewFromFloat(2.0)}.X(), // Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromFloat(99), Price: decimal.NewFromFloat(2.0)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: decimal.NewFromFloat(25), Price: decimal.NewFromFloat(2.2), Comm: decimal.NewFromFloat(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromFloat(100), TotalAcb: decimal.NewFromFloat(302.0)}, Gain: decimal.Zero},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(3.02)}, Gain: decimal.NewFromFloat(-75.479999952)}, // total loss of 100.98, 25.500000048 is superficial
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(28.520000048)}, Gain: decimal.Zero},                     // acb adjust
		TDt{PostSt: TPSS{Shares: decimal.NewFromFloat(26), TotalAcb: decimal.NewFromFloat(85.520000048)}, Gain: decimal.Zero},
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell all
		TTx{TDay: 2, Act: ptf.SELL, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(0.2)}.X(),
		TTx{TDay: 3, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		TTx{TDay: 4, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.2)}.X(),
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(3), Price: decimal.NewFromFloat(0.2)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(12.0)}, Gain: decimal.Zero},
		TDt{PostSt: TPSS{Shares: decimal.NewFromFloat(0), TotalAcb: decimal.NewFromFloat(0)}, Gain: decimal.NewFromFloat(-7)}, // Superficial loss of 3
		TDt{PostSt: TPSS{Shares: decimal.NewFromFloat(0), TotalAcb: decimal.NewFromFloat(3)}, Gain: decimal.Zero},                  // acb adjust
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(10.0)}, Gain: decimal.Zero},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(6.0)}, Gain: decimal.Zero}, // Superficial loss of 3.6
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(9.6)}, Gain: decimal.Zero}, // acb adjust
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal.Zero}, Gain: decimal.NewFromFloat(-9)},
	})

	/*
		buy 10
		sell 5 (gain)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing gain
		TTx{TDay: 2, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(12.0)}, Gain: decimal.Zero},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(6.0)}, Gain: decimal.NewFromFloat(4.0)},
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), SFL: CADSFL(decimal.NewFromFloat(-6.0), false)}.X(),
		// ACB adjust is partial, as if splitting some to another affiliate.
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.02), Curr: EXP_DEFAULT_CURRENCY, FxRate: decimal.NewFromFloat(1.0)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(14.4)}, Gain: decimal.Zero}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.2)}, Gain: decimal.Zero},   // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.3)}, Gain: decimal.Zero},   // sfl ACB adjust 0.02 * 5
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.9)}, Gain: decimal.Zero}, // buy
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), SFL: CADSFL(decimal.NewFromFloat(-0.7), true)}.X(),
		// ACB adjust is partial, as if splitting some to another affiliate.
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.02), Curr: EXP_DEFAULT_CURRENCY, FxRate: decimal.NewFromFloat(1.0)}.X(),
		TTx{TDay: 51, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(14.4)}, Gain: decimal.Zero},                  // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.2)}, Gain: decimal.NewFromFloat(-5.3)}, // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.3)}, Gain: decimal.Zero},                    // sfl ACB adjust 0.02 * 5
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.9)}, Gain: decimal.Zero},                  // buy
	})

	// Un-force the override, and check that we emit an error
	// Expect an error since we did not force.
	txs[1].SpecifiedSuperficialLoss = CADSFL(decimal.NewFromFloat(-0.7), false)
	TxsToDeltaListWithErr(t, txs)

	// Add an un-detectable SFL (ie, the buy occurred in an untracked affiliate)
	/*
		USD SFL test.
		buy 10 (in USD)
		wait
		sell 5 (in USD) (loss)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), Comm: decimal.NewFromFloat(2.0)}.X(),
		// Sell causing superficial loss, because of quick buyback
		TTx{TDay: 50, Act: ptf.SELL, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.2), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.2), SFL: CADSFL(decimal.NewFromFloat(-0.7), true)}.X(),
		// ACB adjust is partial, as if splitting some to another affiliate.
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.02), Curr: ptf.CAD, FxRate: decimal.NewFromFloat(1.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(14.4)}, Gain: decimal.Zero},                  // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.2)}, Gain: decimal.NewFromFloat(-5.3)}, // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(7.3)}, Gain: decimal.Zero},                    // sfl ACB adjust 0.02 * 5
	})

	// Un-force the override, and check that we emit an error
	// Expect an error since we did not force.
	txs[1].SpecifiedSuperficialLoss = CADSFL(decimal.NewFromFloat(-0.7), false)
	TxsToDeltaListWithErr(t, txs)

	// Currency errors
	// Sanity check for ok by itself.
	txs = []*ptf.Tx{
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(0.1), Curr: ptf.CAD, FxRate: decimal.NewFromFloat(1.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal.NewFromFloat(0.1)}, Gain: decimal.Zero},
	})

	txs = []*ptf.Tx{
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(0.1), Curr: EXP_DEFAULT_CURRENCY, FxRate: decimal.NewFromFloat(1.0)}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.Zero, TotalAcb: decimal.NewFromFloat(0.1)}, Gain: decimal.Zero},
	})
	// Non 1.0 exchange rate
	txs = []*ptf.Tx{
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.02), Curr: ptf.USD, FxRate: decimal.NewFromFloat(1.0)}.X(),
	}
	TxsToDeltaListWithErr(t, txs)
	txs = []*ptf.Tx{
		// Non 1.0 exchange rate
		TTx{TDay: 50, Act: ptf.SFLA, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(0.02), Curr: ptf.CAD, FxRate: decimal.NewFromFloat(1.1)}.X(),
	}
	TxsToDeltaListWithErr(t, txs)
}

func TestBasicRocAcbErrors(t *testing.T) {
	// Test that RoC Txs always have zero shares
	sptf := TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(20.0)}.X()
	tx := TTx{Act: ptf.ROC, Shares: decimal.NewFromInt(3), Price: decimal.NewFromFloat(10.0)}.X()
	AddTxWithErr(t, tx, sptf)

	// Test that RoC cannot exceed the current ACB
	sptf = TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(20.0)}.X()
	tx = TTx{Act: ptf.ROC, Price: decimal.NewFromFloat(13.0)}.X()
	AddTxWithErr(t, tx, sptf)

	// Test that RoC cannot occur on registered affiliates, since they have no ACB
	sptf = TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.Null}.X()
	tx = TTx{Act: ptf.ROC, Shares: decimal.Zero, Price: decimal.NewFromFloat(3.0), AffName: "(R)"}.X()
	AddTxWithErr(t, tx, sptf)
}

func TestBasicRocAcb(t *testing.T) {
	// Test basic ROC with different AllAffiliatesShareBalance
	sptf := TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(8), TotalAcb: decimal.NewFromFloat(20.0)}.X()
	tx := TTx{Act: ptf.ROC, Price: decimal.NewFromFloat(1.0)}.X()

	delta := AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(8), TotalAcb: decimal.NewFromFloat(18.0)}, Gain: decimal.Zero})

	// Test RoC with exchange
	sptf = TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(20.0)}.X()
	tx = TTx{Act: ptf.ROC, Price: decimal.NewFromFloat(1.0), FxRate: decimal.NewFromFloat(2.0)}.X()

	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(16.0)}, Gain: decimal.Zero})
}

func TestBasicSflaErrors(t *testing.T) {
	rq := require.New(t)
	// Test than an SfLA on a registered affiliate is invalid
	sptf := TPSS{Shares: decimal.NewFromInt(2), TotalAcb: decimal.Null}.X()
	tx := TTx{Act: ptf.SFLA, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X()
	err := AddTxWithErr(t, tx, sptf)
	rq.Regexp("Registered affiliates do not have an ACB", err)
}

func TestRegisteredAffiliateCapitalGain(t *testing.T) {
	// Test there are no capital gains in registered accounts
	sptf := TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.Null}.X()
	tx := TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(3.0), AffName: "(R)"}.X()
	delta := AddTxNoErr(t, tx, sptf)
	StEq(t, TPSS{Shares: decimal.NewFromInt(3), AcbPerSh: decimal.Null}.X(), delta.PostStatus)
	//RqNaN(t, delta.CapitalGain) TODO

	// Test that we fail if registered account sees non-nan acb
	sptf = TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.Zero}.X()
	tx = TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(3.0), AffName: "(R)"}.X()
	RqPanicsWithRegexp(t, "bad NaN value", func() {
		AddTxWithErr(t, tx, sptf)
	})
	// Same, but non-zero acb
	sptf = TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(1.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(3.0), AffName: "(R)"}.X()
	RqPanicsWithRegexp(t, "bad NaN value", func() {
		AddTxWithErr(t, tx, sptf)
	})
	// Test that non-registered with NaN ACB generates an error as well
	sptf = TPSS{Shares: decimal.NewFromInt(5), TotalAcb: decimal.Null}.X()
	tx = TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(3.0)}.X()
	RqPanicsWithRegexp(t, "bad NaN value", func() {
		AddTxWithErr(t, tx, sptf)
	})
}

func TestAllAffiliateShareBalanceAddTx(t *testing.T) {
	var sptf *ptf.PortfolioSecurityStatus
	var tx *ptf.Tx
	var delta *ptf.TxDelta

	// Basic buy
	sptf = TPSS{Shares: decimal.NewFromInt(3), AllShares: decimal.NewFromInt(7), TotalAcb: decimal.NewFromFloat(15.0)}.X()
	tx = TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(5.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(9), TotalAcb: decimal.NewFromFloat(25.0)}})

	// Basic sell
	sptf = TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(8), AcbPerSh: decimal.NewFromFloat(3.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(5.0)}.X()
	delta = AddTxNoErr(t, tx, sptf)
	ValidateDelta(t, delta,
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(3), AllShares: decimal.NewFromFloat(6.0), AcbPerSh: decimal.NewFromFloat(3.0)}, Gain: decimal.NewFromFloat(4.0)})

	// AllAffiliatesShareBalance too small (error).
	// In theory this could maybe panic, since it should not be possible, but
	// safer and easier to debug if we get a nicer error, which is in the API anyway.
	sptf = TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(15.0)}.X()
	tx = TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(5.0)}.X()
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
		TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(20), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(30), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(40), Price: decimal.NewFromFloat(1.0), AffName: "B (R)"}.X(),
		// Sells
		TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(1.2), AffName: ""}.X(),
		TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.3), AffName: "(R)"}.X(),
		TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(3), Price: decimal.NewFromFloat(1.4), AffName: "B"}.X(), TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(4), Price: decimal.NewFromFloat(1.5), AffName: "B (R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		// Buys
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), AcbPerSh: decimal.NewFromFloat(1.0)}},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(20), AllShares: decimal.NewFromInt(30), TotalAcb: decimal.Null}, Gain: decimal.Null},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(30), AllShares: decimal.NewFromInt(60), AcbPerSh: decimal.NewFromFloat(1.0)}},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(40), AllShares: decimal.NewFromInt(100), TotalAcb: decimal.Null}, Gain: decimal.Null},
		// Sells
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(9), AllShares: decimal.NewFromInt(99), AcbPerSh: decimal.NewFromFloat(1.0)}, Gain: decimal.NewFromFloat(1 * 0.2)},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(18), AllShares: decimal.NewFromInt(97), TotalAcb: decimal.Null}, Gain: decimal.Null},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(27), AllShares: decimal.NewFromInt(94), AcbPerSh: decimal.NewFromFloat(1.0)}, Gain: decimal.NewFromFloat(3 * 0.4)},
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(36), AllShares: decimal.NewFromInt(90), TotalAcb: decimal.Null}, Gain: decimal.Null},
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
		TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{Act: ptf.BUY, Shares: decimal.NewFromInt(20), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		// ROC
		TTx{Act: ptf.ROC, Shares: decimal.Zero, Price: decimal.NewFromFloat(0.2), AffName: "B"}.X(),
		// Sells
		TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.1), AffName: ""}.X(),
		TTx{Act: ptf.SELL, Shares: decimal.NewFromInt(20), Price: decimal.NewFromFloat(1.1), AffName: "B"}.X(),
	}
	deltas := TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		// Buys
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), AcbPerSh: decimal.NewFromFloat(1.0)}}, // Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(20), AllShares: decimal.NewFromInt(30), AcbPerSh: decimal.NewFromFloat(1.0)}}, // B
		// ROC
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(20), AllShares: decimal.NewFromInt(30), AcbPerSh: decimal.NewFromFloat(0.8)}}, // B
		// Sells
		TDt{PostSt: TPSS{Shares: decimal.Zero, AllShares: decimal.NewFromInt(20), AcbPerSh: decimal.Zero}, Gain: decimal.NewFromFloat(10 * 0.1)}, // Default
		TDt{PostSt: TPSS{Shares: decimal.Zero, AllShares: decimal.Zero, AcbPerSh: decimal.Zero}, Gain: decimal.NewFromFloat(20 * 0.3)},                // B
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
	}
	deltas := TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), TotalAcb: decimal.NewFromFloat(5.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(13), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(13), TotalAcb: decimal.NewFromFloat(6.0)}},                                       // Auto-adjust on B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(15), TotalAcb: decimal.NewFromFloat(8.0)}},                                       // B
	})

	/* SFL with all buys on registered affiliate
	   (same txs as above)
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), TotalAcb: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(13), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(15), TotalAcb: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
	})

	/* SFL with all buys on other affiliate B, but sells on a second affiliate (R)
	Make sure it doesn't interfere or cause errors.
	*/
	txs = []*ptf.Tx{
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), TotalAcb: decimal.NewFromFloat(5.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(18), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(18), TotalAcb: decimal.NewFromFloat(6.0)}},                                       // Auto-adjust on B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(8.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(3), AllShares: decimal.NewFromInt(18), TotalAcb: decimal.Null}, Gain: decimal.Null},                                     // Sell in (R)
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(7), Price: decimal.NewFromFloat(1.0), AffName: "C"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "C"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), TotalAcb: decimal.NewFromFloat(5.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.NewFromFloat(7.0)}},                                       // Buy in C
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(5.4375)}},                                    // Auto-adjust on B. Gets 7/16 (43.75%) of the SFL
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(7.5625)}},                                    // Auto-adjust on C. Gets 9/16 (56.25%) of the SFL
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.NewFromFloat(7.4375)}},                                    // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(9), AllShares: decimal.NewFromInt(24), TotalAcb: decimal.NewFromFloat(9.5625)}},                                    // Buy in C
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(7), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), AcbPerSh: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.NewFromFloat(7.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(7.5625)}},                                    // Auto-adjust on B. Gets 9/16 (56.25%) of the SFL
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(9), AllShares: decimal.NewFromInt(24), TotalAcb: decimal.NewFromFloat(9.5625)}},                                    // Buy in B
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(4), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 42, Act: ptf.SELL, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(2.0), AffName: "B"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(5.0)}}, // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.0)}, Gain: decimal.NewFromFloat(-1.0), SFL: decimal.NewFromFloat(-1.0),
			PotentiallyOverAppliedSfl: true}, // SFL of 0.5 * 2(/4) shares
		TDt{PostSt: TPSS{Shares: decimal.Zero, AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.0)}},                                                      // auto adjust on B (100%)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(3.0)}},                                        // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(2), TotalAcb: decimal.NewFromFloat(1.5)}, Gain: decimal.NewFromFloat(0.50)}, // Sell in B
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(4), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 42, Act: ptf.BUY, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(2.0), AffName: ""}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(5.0)}},                                                                              // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.0)}, Gain: decimal.NewFromFloat(-0.5), SFL: decimal.NewFromFloat(-1.5)}, // SFL of 0.5 * 3(/4) shares
		TDt{PostSt: TPSS{Shares: decimal.Zero, AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(0.75)}},                                                                                           // auto adjust on B (50%)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.75)}},                                                                             // auto adjust on default (50%)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(3), TotalAcb: decimal.NewFromFloat(2.75)}},                                                                             // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(4), TotalAcb: decimal.NewFromFloat(3.75)}},                                                                             // Buy in default
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(4), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(5.0)}},                                                                              // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.0)}, Gain: decimal.NewFromFloat(-1.0), SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2(/4) shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(3), TotalAcb: decimal.Null}, Gain: decimal.Null},                                                                            // Buy in B
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(4), Price: decimal.NewFromFloat(0.5), AffName: ""}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 42, Act: ptf.SELL, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(2.0), AffName: "(R)"}.X(),
	}
	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(5), TotalAcb: decimal.NewFromFloat(5.0)}}, // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(1), TotalAcb: decimal.NewFromFloat(1.0)}, Gain: decimal.NewFromFloat(-1.0), SFL: decimal.NewFromFloat(-1.0),
			PotentiallyOverAppliedSfl: true}, // SFL of 0.5 * 2(/4) shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(2), AllShares: decimal.NewFromInt(3), TotalAcb: decimal.Null}, Gain: decimal.Null}, // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(1), AllShares: decimal.NewFromInt(2), TotalAcb: decimal.Null}, Gain: decimal.Null}, // Sell in (R)
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(7), Price: decimal.NewFromFloat(1.0), AffName: "C"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: "", SFL: CADSFL(decimal.NewFromFloat(-1.0), false)}.X(),
		TTx{TDay: 40, Act: ptf.SFLA, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(0.5), AffName: "C"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "C"}.X(),
	}
	deltas := TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), AcbPerSh: decimal.NewFromFloat(1.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.NewFromFloat(7.0)}},                                       // Buy in C
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(7.5)}},                                       // Explicit adjust on C
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), AcbPerSh: decimal.NewFromFloat(1.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(9), AllShares: decimal.NewFromInt(24), TotalAcb: decimal.NewFromFloat(9.5)}},                                       // Buy in C
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
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(10), Price: decimal.NewFromFloat(1.0), AffName: ""}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(5), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 1, Act: ptf.BUY, Shares: decimal.NewFromInt(7), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
		TTx{TDay: 40, Act: ptf.SELL, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(0.5), AffName: "", SFL: CADSFL(decimal.NewFromFloat(-1.0), false)}.X(),
		TTx{TDay: 40, Act: ptf.SFLA, Shares: decimal.NewFromInt(1), Price: decimal.NewFromFloat(0.5), AffName: "B"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "(R)"}.X(),
		TTx{TDay: 41, Act: ptf.BUY, Shares: decimal.NewFromInt(2), Price: decimal.NewFromFloat(1.0), AffName: "B"}.X(),
	}

	deltas = TxsToDeltaListNoErr(t, txs)
	ValidateDeltas(t, deltas, []TDt{
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(10), AllShares: decimal.NewFromInt(10), TotalAcb: decimal.NewFromFloat(10.0)}},                                     // Buy in Default
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(5), AllShares: decimal.NewFromInt(15), AcbPerSh: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.NewFromFloat(7.0)}},                                       // Buy in B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(8), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(8.0)}, SFL: decimal.NewFromFloat(-1.0)}, // SFL of 0.5 * 2 shares
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(20), TotalAcb: decimal.NewFromFloat(7.5)}},                                       // Explicit adjust on B
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(7), AllShares: decimal.NewFromInt(22), TotalAcb: decimal.Null}, Gain: decimal.Null},                                     // Buy in (R)
		TDt{PostSt: TPSS{Shares: decimal.NewFromInt(9), AllShares: decimal.NewFromInt(24), TotalAcb: decimal.NewFromFloat(9.5)}},                                       // Buy in B
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
