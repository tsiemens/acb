package test

import (
	"fmt"
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	ptf "github.com/tsiemens/acb/portfolio"
)

func mkDate(t *testing.T, day uint32) time.Time {
	const tFmt = "2006-01-02"
	tm, err := time.Parse(tFmt, "2017-01-01")
	if err != nil {
		t.Fatal(err)
	}
	return tm.Add(ptf.ONE_DAY_DUR * time.Duration(day))
}

func AssertNil(t *testing.T, o interface{}) {
	if o != nil {
		fmt.Println("Obj was not nil:", o)
		t.FailNow()
	}
}

func AddTxNoErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) *ptf.TxDelta {
	txs := []*ptf.Tx{tx}
	applySuperficialLosses := true
	delta, err := ptf.AddTx(0, txs, preTxStatus, applySuperficialLosses)
	require.Nil(t, err)
	return delta
}

func TestBasicBuyAcb(t *testing.T) {
	rq := require.New(t)

	sptf := ptf.NewEmptyPortfolioSecurityStatus("FOO")
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.BUY,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 3, TotalAcb: 30.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with commission
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.BUY,
		Shares: 2, AmountPerShare: 10.0, Commission: 1.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 21.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with exchange rates
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.BUY,
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
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.SELL,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	txs := []*ptf.Tx{tx}

	delta, err := ptf.AddTx(0, txs, sptf, true)
	rq.Nil(delta)
	rq.NotNil(err)
}

func TestBasicSellAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.SELL,
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
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.SELL,
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
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.SELL,
		Shares: 2, AmountPerShare: 15.0, Commission: 2.0,
		TxCurrency: "XXX", TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: "YYY", CommissionCurrToLocalExchangeRate: 0.4}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 1, TotalAcb: 10.0},
	)
	rq.Equal(delta.CapitalGain, (15.0*2.0*2.0)-20.0-0.8)
}

func TestSuperficialLosses(t *testing.T) {
	rq := require.New(t)

	makeTx := func(day uint32, action ptf.TxAction, shares uint32, amount float64) *ptf.Tx {
		commission := 0.0
		if action == ptf.BUY {
			commission = 2.0
		}
		return &ptf.Tx{Security: "FOO", Date: mkDate(t, day), Action: action,
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

	validate := func(i int, shares uint32, totalAcb float64, gain float64) {
		rq.Equal(
			&ptf.PortfolioSecurityStatus{
				Security:     "FOO",
				ShareBalance: shares,
				TotalAcb:     totalAcb},
			deltas[i].PostStatus,
		)
		rq.Equal(gain, deltas[i].CapitalGain)
	}

	deltas, err = ptf.TxsToDeltaList(txs, nil, true)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, -5)

	/*
		buy 10
		sell 5 (superficial loss)
		sell 4 (superficial loss)
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

	deltas, err = ptf.TxsToDeltaList(txs, nil, true)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 11.0, 0)
	validate(2, 1, 10.2, 0)
	validate(3, 0, 0.0, -10.0)

	/*
		buy 10
		wait
		sell 5 (superficial loss)
		buy 5
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell causing superficial loss, because of quick buyback
	tx1 = makeTx(50, ptf.SELL, 5, 0.2)
	tx2 = makeTx(51, ptf.BUY, 5, 0.2)
	txs = []*ptf.Tx{tx0, tx1, tx2}

	deltas, err = ptf.TxsToDeltaList(txs, nil, true)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 11.0, 0)
	validate(2, 10, 14.0, 0)

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

	deltas, err = ptf.TxsToDeltaList(txs, nil, true)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, -5.0)
	validate(2, 0, 0.0, -5.0)

	/*
		buy 10
		sell 10
		buy 5
		sell 2 (superficial loss)
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

	deltas, err = ptf.TxsToDeltaList(txs, nil, true)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 0, 10.0, 0.0)
	validate(2, 5, 17.0, 0)
	validate(3, 3, 16.599999999999998, 0.0)
	validate(4, 0, 0, -15.999999999999998)

	/*
		buy 10
		sell 5 (gain)
	*/
	tx0 = makeTx(1, ptf.BUY, 10, 1.0)
	// Sell causing gain
	tx1 = makeTx(2, ptf.SELL, 5, 2)
	txs = []*ptf.Tx{tx0, tx1}

	deltas, err = ptf.TxsToDeltaList(txs, nil, true)
	rq.Nil(err)
	validate(0, 10, 12.0, 0)
	validate(1, 5, 6.0, 4.0)
}

func TestBasicRocAcbErrors(t *testing.T) {
	rq := require.New(t)

	// Test that RoC Txs always have zero shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
		Shares: 3, AmountPerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	txs := []*ptf.Tx{tx}

	delta, err := ptf.AddTx(0, txs, sptf, true)
	rq.Nil(delta)
	rq.NotNil(err)

	// Test that RoC cannot exceed the current ACB
	sptf = &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
		Shares: 0, AmountPerShare: 13.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
	txs = []*ptf.Tx{tx}

	delta, err = ptf.AddTx(0, txs, sptf, true)
	rq.Nil(delta)
	rq.NotNil(err)
}

func TestBasicRocAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
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
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
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
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO3", Date: mkDate(t, 3), Action: ptf.BUY, ReadIndex: 1},
		&ptf.Tx{Security: "FOO1", Date: mkDate(t, 1), Action: ptf.BUY, ReadIndex: 2},
	}

	expTxs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO1", Date: mkDate(t, 1), Action: ptf.BUY, ReadIndex: 2},
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO3", Date: mkDate(t, 3), Action: ptf.BUY, ReadIndex: 1},
	}

	ptf.SortTxs(txs, true)
	require.Equal(t, txs, expTxs)
}

func TestTxSort(t *testing.T) {
	txs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO3", Date: mkDate(t, 3), Action: ptf.BUY, ReadIndex: 1},
		&ptf.Tx{Security: "FOO1", Date: mkDate(t, 1), Action: ptf.BUY, ReadIndex: 2},
	}

	expTxs := []*ptf.Tx{
		&ptf.Tx{Security: "FOO1", Date: mkDate(t, 1), Action: ptf.BUY, ReadIndex: 2},
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.SELL, ReadIndex: 0},
		&ptf.Tx{Security: "FOO2", Date: mkDate(t, 2), Action: ptf.BUY, ReadIndex: 3},
		&ptf.Tx{Security: "FOO3", Date: mkDate(t, 3), Action: ptf.BUY, ReadIndex: 1},
	}

	ptf.SortTxs(txs, false)
	require.Equal(t, txs, expTxs)
}
