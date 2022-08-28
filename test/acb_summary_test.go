package test

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	ptf "github.com/tsiemens/acb/portfolio"
)

func makeTxYD(year uint32, dayOfYear int,
	action ptf.TxAction, shares uint32, amount float64) *ptf.Tx {

	commission := 0.0
	if action == ptf.BUY {
		commission = 2.0
	}
	dt := mkDateYD(year, dayOfYear)
	return &ptf.Tx{Security: "FOO", TradeDate: dt.AddDays(-2), SettlementDate: dt, Action: action,
		Shares: shares, AmountPerShare: amount, Commission: commission,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.CAD, CommissionCurrToLocalExchangeRate: 1.0}
}

func makeSflaTxYD(year uint32, dayOfYear int, shares uint32, amount float64) *ptf.Tx {
	dt := mkDateYD(year, dayOfYear)
	return &ptf.Tx{Security: "FOO", TradeDate: dt.AddDays(-2), SettlementDate: dt, Action: ptf.SFLA,
		Shares: shares, AmountPerShare: amount, Commission: 0.0,
		TxCurrency: ptf.CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
		Memo: "automatic SfL ACB adjustment"}
}

func makeSummaryTx(year uint32, dayOfYear int, shares uint32, amount float64) *ptf.Tx {
	dt := mkDateYD(year, dayOfYear)
	return &ptf.Tx{Security: "FOO", TradeDate: dt, SettlementDate: dt, Action: ptf.BUY,
		Shares: shares, AmountPerShare: amount, Commission: 0.0,
		TxCurrency: ptf.DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
		Memo: "Summary"}
}

type SummaryTestHelper struct {
	rq            *require.Assertions
	initialStatus *ptf.PortfolioSecurityStatus
}

func (h *SummaryTestHelper) checkOk(summaryTxs []*ptf.Tx, warnings []string) {
	h.rq.NotNil(summaryTxs)
	h.rq.Nil(warnings)
}

func (h *SummaryTestHelper) checkWarnings(warningCount int, summaryTxs []*ptf.Tx, warnings []string) {
	h.rq.NotNil(warnings)
	h.rq.Equal(warningCount, len(warnings))
}

func (h *SummaryTestHelper) txsToDeltaList(txs []*ptf.Tx) []*ptf.TxDelta {
	deltas, err := ptf.TxsToDeltaList(txs, h.initialStatus, ptf.NewLegacyOptions())
	h.rq.NotNil(deltas)
	h.rq.Nil(err)
	return deltas
}

func TestSummary(t *testing.T) {
	rq := require.New(t)

	// Ensure we don't think we're too close to the summary date.
	date.TodaysDateForTest = date.New(3000, 1, 1)

	initialStatus := &ptf.PortfolioSecurityStatus{
		Security: "FOO",
		// ShareBalance: 0, TotalAcb:     0.0,
	}
	th := SummaryTestHelper{rq, initialStatus}

	// TEST: simple one tx to one summary
	txs := []*ptf.Tx{
		makeTxYD(2021, 4, ptf.BUY, 10, 1.0), // commission 2.0
	}
	expSummaryTxs := []*ptf.Tx{
		makeSummaryTx(2021, 4, 10, 1.2), // commission is added to share ACB
	}

	deltas := th.txsToDeltaList(txs)
	summaryTxs, warnings := ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: nothing at all
	txs = []*ptf.Tx{}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period
	txs = []*ptf.Tx{
		makeTxYD(2022, 4, ptf.BUY, 10, 1.0), // commission 2.0
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period, but there is a close superficial loss
	txs = []*ptf.Tx{
		makeTxYD(2022, 4, ptf.BUY, 10, 1.0),  // commission 2.0
		makeTxYD(2022, 41, ptf.SELL, 5, 0.2), // SFL
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period, but there is a further superficial loss
	txs = []*ptf.Tx{
		makeTxYD(2022, 40, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, 41, ptf.SELL, 5, 0.2), // SFL
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only before period, and there are terminating superficial losses
	txs = []*ptf.Tx{
		makeTxYD(2022, -2, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2), // SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -1, 8, 1.45),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SELL ... 2 days || SFL, BUY ] past
	txs = []*ptf.Tx{
		makeTxYD(2022, -2, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 2, ptf.SELL, 2, 2.0),  // Gain
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -1, 8, 1.45),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 30 days || SELL(+), BUY ] past
	txs = []*ptf.Tx{
		makeTxYD(2022, -2, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 2.0), // Gain
		makeTxYD(2022, 30, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -1, 8, 1.2),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SELL(+), 1 day...  BUY ] past
	// The post SFL will influence the summarizable TXs
	txs = []*ptf.Tx{
		makeTxYD(2022, -2, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 2.0), // Gain
		makeTxYD(2022, 29, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -2, 10, 1.2),
		makeTxYD(2022, -1, ptf.SELL, 2, 2.0), // Gain
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SELL(+), 0 days...  BUY ] past
	// The post SFL will influence the summarizable TXs
	txs = []*ptf.Tx{
		makeTxYD(2022, -1, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 2.0), // Gain
		makeTxYD(2022, 29, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeTxYD(2022, -1, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 2.0), // Gain
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs = []*ptf.Tx{
		makeTxYD(2022, -32, ptf.BUY, 8, 1.0), // commission 2.0
		makeTxYD(2022, -31, ptf.BUY, 2, 1.0), // commission 2.0
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 29, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -32, 8, 1.25),
		makeTxYD(2022, -31, ptf.BUY, 2, 1.0), // ACB of 14 total after here.
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2), // SFL of $2.4
		makeSflaTxYD(2022, -1, 2, 1.2),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || <mix of SFLs, BUYs> ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs = []*ptf.Tx{
		makeTxYD(2022, -71, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -70, ptf.SELL, 2, 0.2), // SFL
		// unsummarizable below
		makeTxYD(2022, -45, ptf.BUY, 8, 1.0),  // commission 2.0
		makeTxYD(2022, -31, ptf.BUY, 2, 1.0),  // commission 2.0
		makeTxYD(2022, -15, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2),  // SFL
		// end of summary period
		makeTxYD(2022, 29, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -70, 8, 1.45),
		makeTxYD(2022, -45, ptf.BUY, 8, 1.0),           // commission 2.0, post ACB = 21.6
		makeTxYD(2022, -31, ptf.BUY, 2, 1.0),           // commission 2.0, post ACB = 25.6
		makeTxYD(2022, -15, ptf.SELL, 2, 0.2),          // SFL of 2.4444444444, ACB = 22.755555556
		makeSflaTxYD(2022, -15, 2, 1.2222222222222223), // ACB of 25.2
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2),           // SFL of 2.75, ACB = 22.05
		makeSflaTxYD(2022, -1, 2, 1.3750000000000002),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 25 days || ... 5 days, SFL, BUY ] past
	txs = []*ptf.Tx{
		makeTxYD(2022, -6, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -5, ptf.SELL, 2, 0.2), // SFL
		// end of summary period
		makeTxYD(2022, 26, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -5, 8, 1.45),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 2 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
	txs = []*ptf.Tx{
		makeTxYD(2022, -33, ptf.BUY, 10, 1.0), // commission 2.0
		// unsummarizable below
		makeTxYD(2022, -20, ptf.BUY, 4, 1.0), // commission 2.0
		makeTxYD(2022, -2, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, -1, ptf.BUY, 2, 0.2),  // commission 2.0
		// end of summary period
		makeTxYD(2022, 2, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 3, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -33, 10, 1.2),
		makeTxYD(2022, -20, ptf.BUY, 4, 1.0), // commission 2.0, ACB = 18
		makeTxYD(2022, -2, ptf.SELL, 2, 0.2), // SFL of 2.171428571, ACB = 15.428571429
		makeSflaTxYD(2022, -2, 2, 1.0857142857142859),
		makeTxYD(2022, -1, ptf.BUY, 2, 0.2), // commission 2.0
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 30 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
	txs = []*ptf.Tx{
		makeTxYD(2022, -33, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -20, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2022, -2, ptf.SELL, 2, 0.2),  // SFL
		makeTxYD(2022, -1, ptf.BUY, 2, 0.6),   // commission 2.0
		// end of summary period
		makeTxYD(2022, 30, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -1, 20, 1.34),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: No shares left in summary.
	txs = []*ptf.Tx{
		makeTxYD(2021, 4, ptf.BUY, 10, 1.0), // commission 2.0
		makeTxYD(2021, 4, ptf.SELL, 10, 1.0),
	}
	expSummaryTxs = []*ptf.Tx{}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: No shares left in summarizable region
	txs = []*ptf.Tx{
		makeTxYD(2022, -33, ptf.BUY, 10, 1.0),  // commission 2.0
		makeTxYD(2022, -33, ptf.SELL, 10, 2.0), // Gain
		// unsummarizable below
		makeTxYD(2022, -20, ptf.BUY, 4, 1.0), // commission 2.0
		// end of summary period
		makeTxYD(2022, 2, ptf.SELL, 2, 0.2), // SFL
		makeTxYD(2022, 3, ptf.BUY, 1, 2.0),  // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeTxYD(2022, -20, ptf.BUY, 4, 1.0), // commission 2.0
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(2, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)
}

func makeSummaryBuyTx(year uint32, shares uint32, amount float64) *ptf.Tx {
	dt := mkDateYD(year, 0)
	return &ptf.Tx{Security: "FOO", TradeDate: dt, SettlementDate: dt, Action: ptf.BUY,
		Shares: shares, AmountPerShare: amount, Commission: 0.0,
		TxCurrency: ptf.DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
		Memo: "Summary base (buy)"}
}

func makeSummaryGainsTx(year uint32, acb float64, gain float64) *ptf.Tx {
	com := 0.0
	amount := acb
	if gain >= 0.0 {
		amount += gain
	} else {
		com = -gain
	}
	dt := mkDateYD(year, 0)
	return &ptf.Tx{Security: "FOO", TradeDate: dt, SettlementDate: dt, Action: ptf.SELL,
		Shares: 1, AmountPerShare: amount, Commission: com,
		TxCurrency: ptf.DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
		Memo: fmt.Sprintf("%d gain summary (sell)", year)}
}

func TestSummaryYearSplits(t *testing.T) {
	rq := require.New(t)

	// Ensure we don't think we're too close to the summary date.
	date.TodaysDateForTest = date.New(3000, 1, 1)

	initialStatus := &ptf.PortfolioSecurityStatus{
		Security: "FOO",
		// ShareBalance: 0, TotalAcb:     0.0,
	}
	th := SummaryTestHelper{rq, initialStatus}

	// TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ... ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs := []*ptf.Tx{
		makeTxYD(2018, 30, ptf.BUY, 8, 1.0),   // commission 2.0
		makeTxYD(2020, 30, ptf.BUY, 8, 1.0),   // commission 2.0
		makeTxYD(2020, 31, ptf.SELL, 1, 2.0),  // GAIN
		makeTxYD(2020, 100, ptf.SELL, 1, 0.9), // LOSS
		makeTxYD(2021, 100, ptf.SELL, 2, 0.2), // LOSS
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2),  // SFL
		makeTxYD(2022, 29, ptf.SELL, 2, 0.2),  // SFL
		makeTxYD(2022, 31, ptf.BUY, 1, 2.0),   // Causes SFL
	}
	summaryAcb := 1.25
	expSummaryTxs := []*ptf.Tx{
		makeSummaryBuyTx(2017, 14, summaryAcb), // shares = final shares (12) + N years with gains (2)
		makeSummaryGainsTx(2020, summaryAcb, 0.4),
		makeSummaryGainsTx(2021, summaryAcb, -2.1),
		makeTxYD(2022, -1, ptf.SELL, 2, 0.2), // SFL
	}

	deltas := th.txsToDeltaList(txs)
	summaryTxs, warnings := ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, true)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)
}
