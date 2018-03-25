package test

import (
	"fmt"
	"testing"
	"time"

	"github.com/stretchr/testify/require"

	ptf "github.com/tsiemens/acb/portfolio"
)

func mkDate(t *testing.T, idx int) time.Time {
	const tFmt = "2006-01-02"
	tm, err := time.Parse(tFmt, fmt.Sprintf("2017-01-%02d", idx))
	if err != nil {
		t.Fatal(err)
	}
	return tm
}

func AssertNil(t *testing.T, o interface{}) {
	if o != nil {
		fmt.Println("Obj was not nil:", o)
		t.FailNow()
	}
}

func AddTxNoErr(t *testing.T, tx *ptf.Tx, preTxStatus *ptf.PortfolioSecurityStatus) *ptf.TxDelta {
	delta, err := ptf.AddTx(tx, preTxStatus)
	require.Nil(t, err)
	return delta
}

func TestBasicBuyAcb(t *testing.T) {
	rq := require.New(t)

	sptf := ptf.NewEmptyPortfolioSecurityStatus("FOO")
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.BUY,
		Shares: 3, PricePerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta := AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 3, TotalAcb: 30.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with commission
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.BUY,
		Shares: 2, PricePerShare: 10.0, Commission: 1.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 21.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)

	// Test with exchange rates
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.BUY,
		Shares: 3, PricePerShare: 12.0, Commission: 1.0,
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
		Shares: 3, PricePerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta, err := ptf.AddTx(tx, sptf)
	rq.Nil(delta)
	rq.NotNil(err)
}

func TestBasicSellAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.SELL,
		Shares: 2, PricePerShare: 15.0, Commission: 0.0,
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
		Shares: 2, PricePerShare: 15.0, Commission: 1.0,
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
		Shares: 2, PricePerShare: 15.0, Commission: 2.0,
		TxCurrency: "XXX", TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: "YYY", CommissionCurrToLocalExchangeRate: 0.4}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 1, TotalAcb: 10.0},
	)
	rq.Equal(delta.CapitalGain, (15.0*2.0*2.0)-20.0-0.8)
}

func TestBasicRocAcbErrors(t *testing.T) {
	rq := require.New(t)

	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
		Shares: 3, PricePerShare: 10.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta, err := ptf.AddTx(tx, sptf)
	rq.Nil(delta)
	rq.NotNil(err)

	sptf = &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx = &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
		Shares: 0, PricePerShare: 13.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta, err = ptf.AddTx(tx, sptf)
	rq.Nil(delta)
	rq.NotNil(err)
}

func TestBasicRocAcb(t *testing.T) {
	rq := require.New(t)

	// Sell all remaining shares
	sptf := &ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 20.0}
	tx := &ptf.Tx{Security: "FOO", Date: mkDate(t, 1), Action: ptf.ROC,
		Shares: 0, PricePerShare: 1.0, Commission: 0.0,
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
		Shares: 0, PricePerShare: 1.0, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 2.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta = AddTxNoErr(t, tx, sptf)
	rq.Equal(delta.PostStatus,
		&ptf.PortfolioSecurityStatus{Security: "FOO", ShareBalance: 2, TotalAcb: 16.0},
	)
	rq.Equal(delta.CapitalGain, 0.0)
}
