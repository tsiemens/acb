package test

import (
	"fmt"
	"testing"

	"github.com/shopspring/decimal"
	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	ptf "github.com/tsiemens/acb/portfolio"
)

func makeSflaTxYD(year uint32, dayOfYear int, amount decimal.Decimal) *ptf.Tx {
	dt := mkDateYD(year, dayOfYear)
	return TTx{TDate: dt.AddDays(-2), SDate: dt, Act: ptf.SFLA, Shares: DInt(1), Price: amount,
		CommCurr: EXP_DEFAULT_CURRENCY, CommFxRate: CURRENCY_RATE_EXPLICIT_ZERO,
		Memo: matchingMemo("Automatic SfL ACB adjustment.*")}.X()
}

type TSimpleSumTx struct {
	Year    uint32
	DoY     int
	Shares  decimal.Decimal
	Amount  decimal.Decimal
	AffName string
}

// eXpand
func (t TSimpleSumTx) X() *ptf.Tx {
	dt := mkDateYD(t.Year, t.DoY)
	return &ptf.Tx{Security: DefaultTestSecurity, TradeDate: dt, SettlementDate: dt, Action: ptf.BUY,
		Shares: t.Shares, AmountPerShare: t.Amount, Commission: decimal.Zero,
		TxCurrency: ptf.DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: decimal.Zero,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: decimal.Zero,
		Memo:      "Summary",
		Affiliate: ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)}
}

type TSumBaseBuyTx struct {
	Year    uint32
	Shares  decimal.Decimal
	Amount  decimal.Decimal
	AffName string
}

// eXpand
func (t TSumBaseBuyTx) X() *ptf.Tx {
	dt := mkDateYD(t.Year, 0)
	// affiliate := ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)
	return &ptf.Tx{Security: DefaultTestSecurity, TradeDate: dt, SettlementDate: dt, Action: ptf.BUY,
		Shares: t.Shares, AmountPerShare: t.Amount, Commission: decimal.Zero,
		TxCurrency: ptf.DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: decimal.Zero,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: decimal.Zero,
		Memo:      "Summary base (buy)",
		Affiliate: ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)}
}

type TSumGainsTx struct {
	Year     uint32
	AcbPerSh decimal.Decimal
	Gain     decimal.Decimal
	AffName  string
}

// eXpand
func (t TSumGainsTx) X() *ptf.Tx {
	com := decimal.Zero
	amount := t.AcbPerSh
	if t.Gain.IsZero() || t.Gain.IsPositive() {
		amount = amount.Add(t.Gain)
	} else {
		com = t.Gain.Neg()
	}
	dt := mkDateYD(t.Year, 0)
	// affiliate := ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)
	return &ptf.Tx{Security: DefaultTestSecurity, TradeDate: dt, SettlementDate: dt, Action: ptf.SELL,
		Shares: DInt(1), AmountPerShare: amount, Commission: com,
		TxCurrency: ptf.DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: decimal.Zero,
		CommissionCurrency: ptf.DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: decimal.Zero,
		Memo:      fmt.Sprintf("%d gain summary (sell)", t.Year),
		Affiliate: ptf.GlobalAffiliateDedupTable.DedupedAffiliate(t.AffName)}
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
		Security: DefaultTestSecurity,
		// ShareBalance: 0, TotalAcb:     decimal.Zero,
	}
	th := SummaryTestHelper{rq, initialStatus}

	// TEST: simple one tx to one summary
	txs := []*ptf.Tx{
		TTx{SYr: 2021, SDoY: 4, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
	}
	expSummaryTxs := []*ptf.Tx{
		TSimpleSumTx{Year: 2021, DoY: 4, Shares: DInt(10), Amount: DFlt(1.2)}.X(), // commission is added to share ACB
	}

	deltas := th.txsToDeltaList(txs)
	summaryTxs, warnings := ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: nothing at all
	txs = []*ptf.Tx{}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: 4, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period, but there is a close superficial loss
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: 4, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: 41, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(), // SFL
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only after summary period, but there is a further superficial loss
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: 40, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: 41, Act: ptf.SELL, Shares: DInt(5), Price: DFlt(0.2)}.X(), // SFL
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)

	// TEST: only before period, and there are terminating superficial losses
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -1, Shares: DInt(8), Amount: DFlt(1.45)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: present [ SELL ... 2 days || SFL, BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
		TTx{SYr: 2022, SDoY: 2, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),    // Gain
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -1, Shares: DInt(8), Amount: DFlt(1.45)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 30 days || SELL(+), BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),               // Gain
		TTx{SYr: 2022, SDoY: 30, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -1, Shares: DInt(8), Amount: DFlt(1.2)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SELL(+), 1 day...  BUY ] past
	// The post SFL will influence the summarizable TXs
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -2, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),               // Gain
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -2, Shares: DInt(10), Amount: DFlt(1.2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(), // Gain
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SELL(+), 0 days...  BUY ] past
	// The post SFL will influence the summarizable TXs
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),               // Gain
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(), // Gain
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -32, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: DInt(2), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -32, Shares: DInt(8), Amount: DFlt(1.25)}.X(),
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: DInt(2), Price: DInt(1), Comm: DInt(2)}.X(),                    // ACB of 14 total after here.
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2), SFL: CADSFL(DFlt(-2.4), false)}.X(), // SFL of $2.4
		makeSflaTxYD(2022, -1, DFlt(2.4)),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: present [ SFL ... 29 days || <mix of SFLs, BUYs> ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -71, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -70, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
		// unsummarizable below
		TTx{SYr: 2022, SDoY: -45, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: DInt(2), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -15, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),  // SFL
		// end of summary period
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -70, Shares: DInt(8), Amount: DFlt(1.45)}.X(),
		TTx{SYr: 2022, SDoY: -45, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),                                    // post ACB = 21.6
		TTx{SYr: 2022, SDoY: -31, Act: ptf.BUY, Shares: DInt(2), Price: DInt(1), Comm: DInt(2)}.X(),                                    // post ACB = 25.6
		TTx{SYr: 2022, SDoY: -15, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2), SFL: CADSFL(DFlt(-2.4444444444444446), false)}.X(), // ACB = 22.755555556
		makeSflaTxYD(2022, -15, DStr("2.4444444444444444")),                                                                            // ACB of 25.2
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2), SFL: CADSFL(DFlt(-2.7500000000000004), false)}.X(),  // ACB = 22.05
		makeSflaTxYD(2022, -1, DFlt(2.75)),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 25 days || ... 5 days, SFL, BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -6, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -5, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
		// end of summary period
		TTx{SYr: 2022, SDoY: 26, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -5, Shares: DInt(8), Amount: DFlt(1.45)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 2 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -34, Act: ptf.BUY, Shares: DInt(1), Price: DInt(1), Comm: DInt(1)}.X(),
		TTx{SYr: 2022, SDoY: -33, Act: ptf.BUY, Shares: DInt(9), Price: DInt(1), Comm: DInt(1)}.X(),
		// unsummarizable below
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: DInt(4), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -2, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(0.2), Comm: DInt(2)}.X(),
		// end of summary period
		TTx{SYr: 2022, SDoY: 2, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 3, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -33, Shares: DInt(10), Amount: DFlt(1.2)}.X(),
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: DInt(4), Price: DInt(1), Comm: DInt(2)}.X(),                                     // ACB = 18
		TTx{SYr: 2022, SDoY: -2, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2), SFL: CADSFL(DStr("-2.1714285714285714"), false)}.X(), // ACB = 15.428571429
		makeSflaTxYD(2022, -2, DStr("2.1714285714285714")),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(0.2), Comm: DInt(2)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: before and after: present [ SFL ... 30 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -33, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -2, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
		TTx{SYr: 2022, SDoY: -1, Act: ptf.BUY, Shares: DInt(2), Price: DFlt(0.6), Comm: DInt(2)}.X(),
		// end of summary period
		TTx{SYr: 2022, SDoY: 30, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2022, DoY: -1, Shares: DFlt(20), Amount: DFlt(1.34)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: No shares left in summary.
	txs = []*ptf.Tx{
		TTx{SYr: 2021, SDoY: 4, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2021, SDoY: 4, Act: ptf.SELL, Shares: DInt(10), Price: DInt(1)}.X(),
	}
	expSummaryTxs = []*ptf.Tx{}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// TEST: No shares left in summarizable region
	txs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -33, Act: ptf.BUY, Shares: DInt(10), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2022, SDoY: -33, Act: ptf.SELL, Shares: DInt(10), Price: DInt(2)}.X(), // Gain
		// unsummarizable below
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: DInt(4), Price: DInt(1), Comm: DInt(2)}.X(),
		// end of summary period
		TTx{SYr: 2022, SDoY: 2, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 3, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	expSummaryTxs = []*ptf.Tx{
		TTx{SYr: 2022, SDoY: -20, Act: ptf.BUY, Shares: DInt(4), Price: DInt(1), Comm: DInt(2)}.X(),
	}

	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, false)
	th.checkWarnings(2, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)
}

func TestSummaryYearSplits(t *testing.T) {
	rq := require.New(t)

	// Ensure we don't think we're too close to the summary date.
	date.TodaysDateForTest = date.New(3000, 1, 1)

	initialStatus := &ptf.PortfolioSecurityStatus{
		Security: DefaultTestSecurity,
		// ShareBalance: 0, TotalAcb:     decimal.Zero,
	}
	th := SummaryTestHelper{rq, initialStatus}

	// TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ... ] past
	// Unsummarizable SFL will push back the summarizing window.
	txs := []*ptf.Tx{
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2020, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2020, SDoY: 31, Act: ptf.SELL, Shares: DInt(1), Price: DInt(2)}.X(),               // GAIN
		TTx{SYr: 2020, SDoY: 100, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(0.9)}.X(),            // LOSS
		TTx{SYr: 2021, SDoY: 100, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),            // LOSS
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 29, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(),             // SFL
		TTx{SYr: 2022, SDoY: 31, Act: ptf.BUY, Shares: DInt(1), Price: DInt(2), Comm: DInt(2)}.X(), // Causes SFL
	}
	summaryAcb := DFlt(1.25)
	expSummaryTxs := []*ptf.Tx{
		TSumBaseBuyTx{Year: 2017, Shares: DFlt(14), Amount: summaryAcb}.X(), // shares = final shares (12) + N years with gains (2)
		TSumGainsTx{Year: 2020, AcbPerSh: summaryAcb, Gain: DFlt(0.4)}.X(),
		TSumGainsTx{Year: 2021, AcbPerSh: summaryAcb, Gain: DFlt(-2.1)}.X(),
		TTx{SYr: 2022, SDoY: -1, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.2)}.X(), // SFL
	}

	deltas := th.txsToDeltaList(txs)
	summaryTxs, warnings := ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, true)
	th.checkWarnings(1, summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)
}

func TestMultiAffiliateSummary(t *testing.T) {
	rq := require.New(t)

	// Ensure we don't think we're too close to the summary date.
	date.TodaysDateForTest = date.New(3000, 1, 1)

	initialStatus := &ptf.PortfolioSecurityStatus{
		Security: DefaultTestSecurity,
	}
	th := SummaryTestHelper{rq, initialStatus}

	// Case: Test basic only buys for each affiliate.
	txs := []*ptf.Tx{
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(4), Price: DInt(1), Comm: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2019, SDoY: 31, Act: ptf.BUY, Shares: DInt(3), Price: DInt(1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 29, Act: ptf.BUY, Shares: DInt(5), Price: DInt(1), Comm: DInt(2), AffName: "B"}.X(),
	}

	// Note these are sorted alphabetically to tiebreak between affiliates
	expSummaryTxs := []*ptf.Tx{
		TSumBaseBuyTx{Year: 2017, Shares: DInt(9), Amount: DFlt(13.0 / 9.0), AffName: "B"}.X(),
		TSumBaseBuyTx{Year: 2017, Shares: DInt(8), Amount: DFlt(10.0 / 8.0)}.X(),
		// Registered accounts use 0 rather than NaN in the summary
		TSumBaseBuyTx{Year: 2017, Shares: DInt(3), Amount: decimal.Zero, AffName: "(R)"}.X(),
	}

	deltas := th.txsToDeltaList(txs)

	summaryTxs, warnings := ptf.MakeSummaryTxs(mkDateYD(2022, -1), deltas, true)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Test capital gains in multiple years, different between affiliates.
	txs = []*ptf.Tx{
		// Buys
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2019, SDoY: 31, Act: ptf.BUY, Shares: DInt(6), Price: DFlt(1.2), Comm: DInt(2), AffName: "(R)"}.X(),
		// Sells
		TTx{SYr: 2019, SDoY: 5, Act: ptf.SELL, Shares: DInt(1), Price: DInt(2)}.X(),
		TTx{SYr: 2019, SDoY: 6, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.1), AffName: "B"}.X(),
		TTx{SYr: 2020, SDoY: 7, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.2), AffName: "(R)"}.X(),
		TTx{SYr: 2021, SDoY: 7, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.3)}.X(),
		TTx{SYr: 2022, SDoY: 7, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.4), AffName: "B"}.X(),
		TTx{SYr: 2022, SDoY: 8, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(2.5), AffName: "(R)"}.X(),
	}

	defShareAcb := DFlt(10.0 / 8.0)
	bShareAcb := DFlt(9.7 / 7.0)
	expSummaryTxs = []*ptf.Tx{
		TSumBaseBuyTx{Year: 2017, Shares: DInt(7), Amount: bShareAcb, AffName: "B"}.X(),
		TSumBaseBuyTx{Year: 2017, Shares: DInt(8), Amount: defShareAcb}.X(),
		TSumBaseBuyTx{Year: 2017, Shares: DInt(4), Amount: decimal.Zero, AffName: "(R)"}.X(), // No gains on (R), so only the base

		TSumGainsTx{Year: 2019, AcbPerSh: bShareAcb, Gain: DFlt(2.1).Sub(bShareAcb), AffName: "B"}.X(),
		TSumGainsTx{Year: 2019, AcbPerSh: defShareAcb, Gain: DFlt(2.0).Sub(defShareAcb)}.X(),
		TSumGainsTx{Year: 2021, AcbPerSh: defShareAcb, Gain: DFlt(2.3).Sub(defShareAcb)}.X(),
		TSumGainsTx{Year: 2022, AcbPerSh: bShareAcb, Gain: DFlt(2.4).Sub(bShareAcb), AffName: "B"}.X(),
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2024, -1), deltas, true)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: only some affiliates have gains (registered & non-registered)
	txs = []*ptf.Tx{
		// Buys
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2019, SDoY: 31, Act: ptf.BUY, Shares: DInt(6), Price: DFlt(1.2), Comm: DInt(2), AffName: "(R)"}.X(),
		// Sells
		TTx{SYr: 2019, SDoY: 5, Act: ptf.SELL, Shares: DInt(1), Price: DInt(2)}.X(),
	}

	defShareAcb = DFlt(10.0 / 8.0)
	bShareAcb = DFlt(9.7 / 7.0)
	expSummaryTxs = []*ptf.Tx{
		TSumBaseBuyTx{Year: 2017, Shares: DInt(7), Amount: bShareAcb, AffName: "B"}.X(),
		TSumBaseBuyTx{Year: 2017, Shares: DInt(8), Amount: defShareAcb}.X(),
		TSumBaseBuyTx{Year: 2017, Shares: DInt(6), Amount: decimal.Zero, AffName: "(R)"}.X(), // No gains on (R), so only the base

		TSumGainsTx{Year: 2019, AcbPerSh: defShareAcb, Gain: DFlt(2.0).Sub(defShareAcb)}.X(),
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2024, -1), deltas, true)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Simple summary, where some affiliates have sells
	txs = []*ptf.Tx{
		// Buys
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2019, SDoY: 31, Act: ptf.BUY, Shares: DInt(6), Price: DFlt(1.2), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2019, SDoY: 40, Act: ptf.BUY, Shares: DInt(5), Price: DFlt(1.3), Comm: DInt(2), AffName: "B (R)"}.X(),
		// Sells
		TTx{SYr: 2020, SDoY: 5, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2020, SDoY: 6, Act: ptf.SELL, Shares: DInt(3), Price: DInt(2), AffName: "B (R)"}.X(),
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2018, DoY: 30, Shares: DInt(8), Amount: DFlt(10.0 / 8.0)}.X(),
		TSimpleSumTx{Year: 2019, DoY: 31, Shares: DInt(6), Amount: decimal.Zero, AffName: "(R)"}.X(),
		TSimpleSumTx{Year: 2020, DoY: 5, Shares: DInt(5), Amount: DFlt(9.7 / 7), AffName: "B"}.X(),
		TSimpleSumTx{Year: 2020, DoY: 6, Shares: DInt(2), Amount: decimal.Zero, AffName: "B (R)"}.X(),
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2024, -1), deltas, false /* year gains*/)
	th.checkOk(summaryTxs, warnings)
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Some affiliates net zero shares at the end
	txs = []*ptf.Tx{
		// Buys
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2019, SDoY: 31, Act: ptf.BUY, Shares: DInt(6), Price: DFlt(1.2), Comm: DInt(2), AffName: "(R)"}.X(),
		// Sells
		TTx{SYr: 2019, SDoY: 5, Act: ptf.SELL, Shares: DInt(7), Price: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2019, SDoY: 5, Act: ptf.SELL, Shares: DInt(6), Price: DInt(2), AffName: "(R)"}.X(),
	}

	bShareAcb = DFlt(9.7 / 7.0)
	expSummaryTxs = []*ptf.Tx{
		TSumBaseBuyTx{Year: 2017, Shares: DInt(1), Amount: decimal.Zero, AffName: "B"}.X(),
		TSumBaseBuyTx{Year: 2017, Shares: DInt(8), Amount: DFlt(10.0 / 8)}.X(),

		TSumGainsTx{Year: 2019, AcbPerSh: decimal.Zero, Gain: DFlt(2.0).Sub(bShareAcb).Mul(DFlt(7.0)), AffName: "B"}.X(),
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2024, -1), deltas, true)
	th.checkWarnings(1, summaryTxs, warnings) // zero warning
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Superficial losses in one affiliate, and another affiliate with zero Txs
	//			before the summarizable range.
	txs = []*ptf.Tx{
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "B"}.X(),
		TTx{SYr: 2020, SDoY: 5, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2), AffName: "B"}.X(),
		// ^^ Summarizable period ^^
		TTx{SYr: 2020, SDoY: 101, Act: ptf.BUY, Shares: DInt(2), Price: DInt(1), AffName: "B"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(1), AffName: "C"}.X(),
		// ^^ Requested summary period ^^
		TTx{SYr: 2020, SDoY: 105, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.9), SFL: CADSFL(DFlt(-0.751020), false), AffName: "B"}.X(),
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2018, DoY: 30, Shares: DInt(8), Amount: DFlt(10.0 / 8.0)}.X(),
		TSimpleSumTx{Year: 2020, DoY: 5, Shares: DInt(5), Amount: DFlt(9.7 / 7), AffName: "B"}.X(),
		// ^^ Summarizable period ^^
		TTx{SYr: 2020, SDoY: 101, Act: ptf.BUY, Shares: DInt(2), Price: DInt(1), AffName: "B"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(1), AffName: "C"}.X(),
		// ^^ Requested summary period ^^
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2020, 104), deltas, false /* year gains*/)
	th.checkWarnings(1, summaryTxs, warnings) // zero warning
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Superficial loss after summary period, and presence of registered
	//       affiliate (where all of their Deltas have a SuperficialLoss of NaN)
	txs = []*ptf.Tx{
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 59, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),
		TTx{SYr: 2020, SDoY: 60, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		// ^^ Summarizable period ^^
		TTx{SYr: 2020, SDoY: 102, Act: ptf.SELL, Shares: DInt(3), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(2)}.X(),
		// ^^ Requested summary period ^^
		TTx{SYr: 2020, SDoY: 105, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.9), SFL: CADSFL(DFlt(-1.2), false)}.X(),
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2020, DoY: 59, Shares: DInt(6), Amount: DFlt(10.0 / 8.0)}.X(),
		TSimpleSumTx{Year: 2020, DoY: 60, Shares: DInt(5), Amount: decimal.Zero, AffName: "(R)"}.X(),
		// ^^ Summarizable period ^^
		TTx{SYr: 2020, SDoY: 102, Act: ptf.SELL, Shares: DInt(3), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(2)}.X(),
		// ^^ Requested summary period ^^
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2020, 104), deltas, false /* year gains*/)
	th.checkWarnings(1, summaryTxs, warnings) // zero warning
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Superficial loss after summary period, and presence of registered
	//       affiliate (where all of their Deltas have a SuperficialLoss of NaN) sales
	//			at least every 30 days until beginning of time. (verifies that the
	//			summarizable period doesn't keep getting pushed backwards).
	txs = []*ptf.Tx{
		TTx{SYr: 2020, SDoY: 50, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 51, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2020, SDoY: 59, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),
		TTx{SYr: 2020, SDoY: 60, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 70, Act: ptf.SELL, Shares: DInt(1), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		// ^^ Summarizable period ^^
		TTx{SYr: 2020, SDoY: 85, Act: ptf.SELL, Shares: DInt(3), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(2)}.X(),
		// ^^ Requested summary period ^^
		TTx{SYr: 2020, SDoY: 105, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(0.9), SFL: CADSFL(DFlt(-1.2), false)}.X(),
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2020, DoY: 59, Shares: DInt(6), Amount: DFlt(10.0 / 8.0)}.X(),
		TSimpleSumTx{Year: 2020, DoY: 70, Shares: DInt(5), Amount: decimal.Zero, AffName: "(R)"}.X(),
		// ^^ Summarizable period ^^
		TTx{SYr: 2020, SDoY: 85, Act: ptf.SELL, Shares: DInt(3), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(2)}.X(),
		// ^^ Requested summary period ^^
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2020, 104), deltas, false /* year gains*/)
	th.checkWarnings(1, summaryTxs, warnings) // zero warning
	ValidateTxs(t, expSummaryTxs, summaryTxs)

	// Case: Only registered sales after summary period. Verify not treated as
	//			superficial losses (because their SuperficialLoss is NaN).
	txs = []*ptf.Tx{
		TTx{SYr: 2018, SDoY: 30, Act: ptf.BUY, Shares: DInt(8), Price: DInt(1), Comm: DInt(2)}.X(),
		TTx{SYr: 2018, SDoY: 31, Act: ptf.BUY, Shares: DInt(7), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 59, Act: ptf.SELL, Shares: DInt(2), Price: DInt(2)}.X(),
		TTx{SYr: 2020, SDoY: 60, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 102, Act: ptf.SELL, Shares: DInt(3), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
		TTx{SYr: 2020, SDoY: 103, Act: ptf.BUY, Shares: DInt(3), Price: DInt(2)}.X(),
		// ^^ Requested summary period ^^
		TTx{SYr: 2020, SDoY: 105, Act: ptf.SELL, Shares: DInt(2), Price: DFlt(1.1), Comm: DInt(2), AffName: "(R)"}.X(),
	}
	expSummaryTxs = []*ptf.Tx{
		TSimpleSumTx{Year: 2020, DoY: 102, Shares: DInt(2), Amount: decimal.Zero, AffName: "(R)"}.X(),
		TSimpleSumTx{Year: 2020, DoY: 103, Shares: DInt(9), Amount: DFlt(1.5)}.X(),
		// ^^ Summarizable period ^^
		// ^^ Requested summary period ^^
	}
	deltas = th.txsToDeltaList(txs)
	summaryTxs, warnings = ptf.MakeSummaryTxs(mkDateYD(2020, 104), deltas, false /* year gains*/)
	th.checkOk(summaryTxs, warnings) // zero warning
	ValidateTxs(t, expSummaryTxs, summaryTxs)
}
