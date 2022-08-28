package test

import (
	"fmt"
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	ptf "github.com/tsiemens/acb/portfolio"
)

func mkDateYD(year uint32, day int) date.Date {
	tm := date.New(year, time.January, 1)
	return tm.AddDays(day)
}

func mkDate(day int) date.Date {
	return mkDateYD(2017, day)
}

func AlmostEqual(t *testing.T, exp float64, actual float64) {
	diff := exp - actual
	if diff < 0.0000001 && diff > -0.0000001 {
		return
	}
	require.Equal(t, exp, actual)
	t.Fatal(fmt.Errorf("%f was not almost equal %f (expected)\n", actual, exp))
	t.FailNow()
}

func AddTxNoErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) *ptf.TxDelta {
	txs := []*ptf.Tx{tx}
	plo := ptf.NewLegacyOptions()
	delta, newTx, err := ptf.AddTx(0, txs, preTxStatus, plo)
	require.Nil(t, newTx)
	require.Nil(t, err)
	return delta
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
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 3, TotalAcb: 30.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with commission
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.BUY,
		Shares: 2, AmountPerShare: 10.0, Commission: 1.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 21.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with exchange rates
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.BUY,
		Shares: 3, AmountPerShare: 12.0, Commission: 1.0,
		TxCurrency: ptf.USD, TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: "XXX", CommissionCurrToLocalExchangeRate: 0.3}

	delta = AddTxNoErr(t, tx, delta.PostStatus)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 5,
			TotalAcb: 21.0 + (2 * 36.0) + 0.3},
	)
	rq.Equal(delta.CapitalGain, 0.0)
}

func TestBasicSellAcbErrors(t *testing.T) {
	rq := require.New(t)

	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	txs := []*ptf.Tx{tx}

	plo := ptf.NewLegacyOptions()
	delta, newTx, err := ptf.AddTx(0, txs, sptf, plo)
	rq.Nil(newTx)
	rq.Nil(delta)
	rq.NotNil(err)
}

func TestBasicSellAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 0, TotalAcb: 0.0},
	)
	rq.Equal(delta.CapitalGain, 10.0)

	// Sell shares with commission
	sptf = &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 3, TotalAcb: 30.0}
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 1.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 1, TotalAcb: 10.0},
	)
	rq.Equal(delta.CapitalGain, 9.0)

	// Sell shares with exchange rate
	sptf = &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 3, TotalAcb: 30.0}
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 2.0,
		TxCurrency: "XXX", TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: "YYY", CommissionCurrToLocalExchangeRate: 0.4}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 1, TotalAcb: 10.0},
	)
	rq.Equal(delta.CapitalGain, (15.0*2.0*2.0)-20.0-0.8)
}

func doTestSuperficialLosses(t *testing.T, partialLosses bool) {
	rq := require.New(t)

	makeTx := func(day int, action ptf.TxAction, shares uint32, amount float64) *ptf.Tx {
		commission := 0.0
		if action == ptf.BUY {
			commission = 2.0
		}
		return &ptf.Tx{Security: "FOO", SettlementDate: mkDate(day), Action: action,
			Shares: shares, AmountPerShare: amount, Commission: commission,
			TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
			CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
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
			&ptf.PortfolioSecurityStatus{
				Security:     "FOO",
				ShareBalance: shareBalance,
				TotalAcb:     deltas[i].PostStatus.TotalAcb},
			deltas[i].PostStatus,
		)
		AlmostEqual(t, gain, deltas[i].CapitalGain)
	}

	plo := ptf.LegacyOptions{
		NoSuperficialLosses:        false,
		NoPartialSuperficialLosses: !partialLosses,
	}

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
	if partialLosses {
		validate(1, 5, 6.0, -4.0) // $1 superficial
		validate(2, 5, 7.0, 0.0)  // acb adjust
		validate(3, 1, 1.4, -3.6) // $1.2 superficial
		validate(4, 1, 2.6, 0.0)  // acb adjust
		validate(5, 0, 0.0, -2.4)
	} else {
		validate(1, 5, 6.0, 0)  // $5 sfl
		validate(2, 5, 11.0, 0) // Loss ACB adjust
		validate(3, 1, 2.2, 0)  // $8 sfl
		validate(4, 1, 10.2, 0) // Loss ACB adjust
		validate(5, 0, 0.0, -10.0)
	}

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
	if partialLosses {
		validate(1, 1, 3.02, -75.479999952) // total loss of 100.98, 25.500000048 is superficial
		validate(2, 1, 28.520000048, 0.0)   // acb adjust
		validate(3, 26, 85.520000048, 0)
	} else {
		validate(1, 1, 3.02, 0) // total superfical loss of 100.98
		validate(2, 1, 104, 0)  // sfl ACB adjust
		validate(3, 26, 161, 0)
	}

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
	if partialLosses {
		validate(1, 0, 0, -7)  // Superficial loss of 3
		validate(2, 0, 3, 0.0) // acb adjust
		validate(3, 5, 10.0, 0)
		validate(4, 3, 6.0, 0.0) // Superficial loss of 3.6
		validate(5, 3, 9.6, 0.0) // acb adjust
		validate(6, 0, 0, -9)
	} else {
		validate(1, 0, 0.0, 0.0)  // sfl
		validate(2, 0, 10.0, 0.0) // acb adjust
		validate(3, 5, 17.0, 0)
		validate(4, 3, 10.2, 0.0)               // sfl
		validate(5, 3, 16.599999999999998, 0.0) // acb adjust
		validate(6, 0, 0, -15.999999999999998)
	}

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
}

func TestSuperficialLosses(t *testing.T) {
	doTestSuperficialLosses(t, true)
}

func TestSuperficialLossesWithoutPartials(t *testing.T) {
	doTestSuperficialLosses(t, false)
}

func TestBasicRocAcbErrors(t *testing.T) {
	rq := require.New(t)

	// Test that RoC Txs always have zero shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	txs := []*ptf.Tx{tx}

	plo := ptf.NewLegacyOptions()
	delta, newTx, err := ptf.AddTx(0, txs, sptf, plo)
	rq.Nil(delta)
	rq.Nil(newTx)
	rq.NotNil(err)

	// Test that RoC cannot exceed the current ACB
	sptf = &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 13.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	txs = []*ptf.Tx{tx}

	delta, newTx, err = ptf.AddTx(0, txs, sptf, plo)
	rq.Nil(delta)
	rq.Nil(newTx)
	rq.NotNil(err)
}

func TestBasicRocAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 1.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 18.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test RoC with exchange
	sptf = &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx = &ptf.Tx{Security: "FOO", SettlementDate: mkDate(1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 1.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 16.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)
}

func TestTxSortLegacySortBuysBeforeSells(t *testing.T) {
	txs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO3", SettlementDate: mkDate(3), Action: ptf.BUY, ReadIndex: 1},
		&ptf.Tx{Security: "FOO1", SettlementDate: mkDate(1), Action: ptf.BUY, ReadIndex: 2},
	}

	expTxs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO1", SettlementDate: mkDate(1), Action: ptf.BUY, ReadIndex: 2},
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO2", SettlementDate: mkDate(2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO3", SettlementDate: mkDate(3), Action: ptf.BUY, ReadIndex: 1},
	}

	ptf.SortTxs(txs, true)
	require.Equal(t, txs, expTxs)
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

	ptf.SortTxs(txs, false)
	require.Equal(t, txs, expTxs)
}
