package test

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	ptf "github.com/tsiemens/acb/portfolio"
)

func makeSflaTxYD(year uint32, dayOfYear int, shares uint32, amount float64) *ptf.Tx {
	dt := mkDateYD(year, dayOfYear)
	return TTx{TDate: dt.AddDays(-2), SDate: dt, Act: ptf.SFLA, Shares: shares, Price: amount,
		CommCurr: EXP_DEFAULT_CURRENCY, CommFxRate: EXP_FLOAT_ZERO,
		Memo: "automatic SfL ACB adjustment"}.X()
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
	// Ensure default affiliate exists
	ptf.GlobalAffiliateDedupTable.DedupedAffiliate("")

	// Ensure we don't think we're too close to the summary date.
	date.TodaysDateForTest = date.New(3000, 1, 1)

	initialStatus := &ptf.PortfolioSecurityStatus{
		Security: "FOO",
		// ShareBalance: 0, TotalAcb:     0.0,
	}
	th := SummaryTestHelper{rq, initialStatus}

	// TEST: simple one tx to one summary
	txs := []*ptf.Tx{
		TTx{SYr: 2021, SDoY: 4, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
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
		TTx{SYr: 2022, SDoY: 4, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period, but there is a close superficial loss
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: 4, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: 41, Act: ptf.SELL, Shares: 5, Price: 0.2}.X(), // SFL
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period, but there is a further superficial loss
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: 40, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: 41, Act: ptf.SELL, Shares: 5, Price: 0.2}.X(), // SFL
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only before period, and there are terminating superficial losses
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
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
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
		TTx{SYr: 2022, SDoY: 2, Act: ptf.SELL, Shares: 2, Price: 2.0}.X(),  // Gain
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
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 2.0}.X(),           // Gain
		TTx{SYr: 2022, SDoY: 30, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
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
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 2.0}.X(),           // Gain
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -2, 10, 1.2),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 2.0}.X(), // Gain
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SELL(+), 0 days...  BUY ] past
	// The post SFL will influence the summarizable TXs
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 2.0}.X(),           // Gain
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 2.0}.X(), // Gain
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -32, Act: ptf.BUY, Shares: 8, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: 2, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -32, 8, 1.25),
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: 2, Price: 1.0, Comm: 2.0}.X(), // ACB of 14 total after here.
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),            // SFL of $2.4
		makeSflaTxYD(2022, -1, 2, 1.2),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || <mix of SFLs, BUYs> ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -71, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -70, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
		// unsummarizable below
		TTx{SYr: 2022, SDoY: -45, Act: ptf.BUY, Shares: 8, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: 2, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -15, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),  // SFL
		// end of summary period
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -70, 8, 1.45),
		TTx{SYr: 2022, SDoY: -45, Act: ptf.BUY, Shares: 8, Price: 1.0, Comm: 2.0}.X(), // post ACB = 21.6
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: 2, Price: 1.0, Comm: 2.0}.X(), // post ACB = 25.6
		TTx{SYr: 2022, SDoY: -15, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL of 2.4444444444, ACB = 22.755555556
		makeSflaTxYD(2022, -15, 2, 1.2222222222222223),                                // ACB of 25.2
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),            // SFL of 2.75, ACB = 22.05
		makeSflaTxYD(2022, -1, 2, 1.3750000000000002),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 25 days || ... 5 days, SFL, BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -6, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -5, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
		// end of summary period
		TTx{SYr: 2022, SDoY: 26, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
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
		TTx{SYr: 2022, SDoY: -33, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		// unsummarizable below
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: 4, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -2, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: 2, Price: 0.2, Comm: 2.0}.X(),
		// end of summary period
		TTx{SYr: 2022, SDoY: 2, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 3, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		makeSummaryTx(2022, -33, 10, 1.2),
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: 4, Price: 1.0, Comm: 2.0}.X(), // ACB = 18
		TTx{SYr: 2022, SDoY: -2, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),            // SFL of 2.171428571, ACB = 15.428571429
		makeSflaTxYD(2022, -2, 2, 1.0857142857142859),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: 2, Price: 0.2, Comm: 2.0}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 30 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -33, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -2, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: 2, Price: 0.6, Comm: 2.0}.X(),
		// end of summary period
		TTx{SYr: 2022, SDoY: 30, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
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
		TTx{SYr: 2021, SDoY: 4, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2021, SDoY: 4, Act: ptf.SELL, Shares: 10, Price: 1.0}.X(),
	}
	expSummaryTxs = []*ptf.Tx{}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)

	// TEST: No shares left in summarizable region
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -33, Act: ptf.BUY, Shares: 10, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2022, SDoY: -33, Act: ptf.SELL, Shares: 10, Price: 2.0}.X(), // Gain
		// unsummarizable below
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: 4, Price: 1.0, Comm: 2.0}.X(),
		// end of summary period
		TTx{SYr: 2022, SDoY: 2, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 3, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: 4, Price: 1.0, Comm: 2.0}.X(),
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
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: 8, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2020, SDoY: 30, Act: ptf.BUY, Shares: 8, Price: 1.0, Comm: 2.0}.X(),
		TTx{SYr: 2020, SDoY: 31, Act: ptf.SELL, Shares: 1, Price: 2.0}.X(),           // GAIN
		TTx{SYr: 2020, SDoY: 100, Act: ptf.SELL, Shares: 1, Price: 0.9}.X(),          // LOSS
		TTx{SYr: 2021, SDoY: 100, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),          // LOSS
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(),           // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: 1, Price: 2.0, Comm: 2.0}.X(), // Causes SFL
	}
	summaryAcb := 1.25
	expSummaryTxs := []*ptf.Tx{
		makeSummaryBuyTx(2017, 14, summaryAcb), // shares = final shares (12) + N years with gains (2)
		makeSummaryGainsTx(2020, summaryAcb, 0.4),
		makeSummaryGainsTx(2021, summaryAcb, -2.1),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: 2, Price: 0.2}.X(), // SFL
	}

	deltas := th.txsToDeltaList(txs)
	summaryTxs, warnings := ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, true)
	th.checkWarnings(1, summaryTxs, warnings)
	rq.Equal(expSummaryTxs, summaryTxs)
}

/*

Functionality TODO
- Summary TX generation for multiple affiliates

*/
