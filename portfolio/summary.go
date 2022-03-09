package portfolio

import (
	"sort"
	"time"

	"github.com/tsiemens/acb/date"
)

// Return a slice of Txs which can summarise all txs in `deltas` up to `latestDate`.
// Multiple Txs might be returned if it is not possible to accurately summarise
// in a single Tx without altering superficial losses (and preserving overall
// capital gains?)
//
// Note that `deltas` should provide all TXs for 60 days after latestDate, otherwise
// the summary may become innacurate/problematic if a new TX were added within
// that 60 day period after, and introduced a new superficial loss within 30 days
// of the summary.
//
// TODO do we want an option to break summaries up so they are per-year? (so we
// also still get a per-year capital gains summary, so we can look backwards?)
//
// eg 1. (cannot be summarized)
// 2021-11-05 BUY  1  @ 1.50
// 2021-12-05 BUY  11 @ 1.50
// 2022-01-01 SELL 10 @ 1.00
//
// Return: summary Txs, user warnings, error
func MakeSummaryTxs(latestDate date.Date, deltas []*TxDelta) ([]*Tx, []string) {
	// Step 1: Find the latest Delta <= latestDate
	latestDeltaInSummaryRangeIdx := -1
	for i, delta := range deltas {
		if delta.Tx.Date.After(latestDate) {
			break
		}
		latestDeltaInSummaryRangeIdx = i
	}
	if latestDeltaInSummaryRangeIdx == -1 {
		return nil, []string{"No transactions in the summary period"}
	}

	// Step 2: determine if any of the TXs within 30 days of latestDate are
	// superficial losses.
	// If any are, save the date 30 days prior to it (firstSuperficialLossPeriodDay)
	txInSummaryOverlapsSuperficialLoss := false
	firstSuperficialLossPeriodDay := date.New(3000, time.January, 1)
	latestInSummaryDate := deltas[latestDeltaInSummaryRangeIdx].Tx.Date
	for _, delta := range deltas[latestDeltaInSummaryRangeIdx+1:] {
		if delta.SuperficialLoss != 0.0 {
			firstSuperficialLossPeriodDay = GetFirstDayInSuperficialLossPeriod(delta.Tx.Date)
			txInSummaryOverlapsSuperficialLoss = !latestInSummaryDate.Before(firstSuperficialLossPeriodDay)
			break
		}
	}

	// Step 3: Find the latest TX in the summary period that can't affect any
	// unsummarized superficial losses.
	latestSummarizableDeltaIdx := -1
	if txInSummaryOverlapsSuperficialLoss {
		// Find the txs which we wanted to summarize, but can't because they can affect
		// this superficial loss' partial calculation.
		// This will be any tx within the 30 day period of the first superficial loss
		// after the summary boundary, but also every tx within the 30 period
		// of any superficial loss at the end of the summary range.
		for i := latestDeltaInSummaryRangeIdx; i >= 0; i-- {
			delta := deltas[i]
			if delta.Tx.Date.Before(firstSuperficialLossPeriodDay) {
				latestSummarizableDeltaIdx = i
				break
			}
			if delta.SuperficialLoss != 0.0 {
				// We've encountered another superficial loss within the summary
				// range. This can be affected by previous txs, so we need to now push
				// up the period where we can't find any txs.
				firstSuperficialLossPeriodDay = GetFirstDayInSuperficialLossPeriod(delta.Tx.Date)
			}
		}
	} else {
		latestSummarizableDeltaIdx = latestDeltaInSummaryRangeIdx
	}

	var warnings []string
	summaryPeriodTxs := []*Tx{}

	if latestSummarizableDeltaIdx != -1 {

		tx := deltas[latestSummarizableDeltaIdx].Tx
		// All one TX. No capital gains yet.
		sumPostStatus := deltas[latestSummarizableDeltaIdx].PostStatus
		if sumPostStatus.ShareBalance != 0 {
			summaryTx := &Tx{
				Security: tx.Security, Date: tx.Date, Action: BUY,
				Shares:         sumPostStatus.ShareBalance,
				AmountPerShare: sumPostStatus.TotalAcb / float64(sumPostStatus.ShareBalance),
				Commission:     0.0,
				TxCurrency:     DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
				CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
				Memo:      "Summary",
				ReadIndex: 0, // This needs to be the first Tx in the list.
			}

			summaryPeriodTxs = append(summaryPeriodTxs, summaryTx)
		} else {
			warnings = append(warnings, "Share balance at the end of the summarized period was zero")
		}

		// TODO Create a summary Sell TX with the same cap gains for the year?
	}

	unsummarizableTxs := deltas[latestSummarizableDeltaIdx+1 : latestDeltaInSummaryRangeIdx+1]
	if len(unsummarizableTxs) > 0 {
		warnings = append(warnings, "Some transactions to be summarized could not be due to superficial-loss conflicts")
	}
	for _, delta := range unsummarizableTxs {
		summaryPeriodTxs = append(summaryPeriodTxs, delta.Tx)
	}

	today := date.Today()
	if latestDeltaInSummaryRangeIdx != -1 {
		lastSummarizableDelta := deltas[latestDeltaInSummaryRangeIdx]
		// Find the very latest day that could possibly ever affect or be affected by
		// the last tx. This should be 60 days.
		lastAffectingDay := GetLastDayInSuperficialLossPeriod(
			GetLastDayInSuperficialLossPeriod(lastSummarizableDelta.Tx.Date))
		if !today.After(lastAffectingDay) {
			warnings = append(warnings,
				"The current date is such that new TXs could potentially alter how the "+
					"summary is created. You should wait 60 days after your latest "+
					"transaction within the summary period to generate the summary")
		}
	}

	return summaryPeriodTxs, warnings
}

type CollectedSummaryData struct {
	Txs []*Tx
	// Warnings -> list of secs that encountered this warning
	Warnings map[string][]string
	// Security -> errors encountered (populated externally)
	Errors map[string][]error
}

func MakeAggregateSummaryTxs(
	latestDate date.Date,
	deltasBySec map[string][]*TxDelta) *CollectedSummaryData {

	allSummaryTxs := []*Tx{}
	// Warnings -> list of secs that encountered this warning.
	allWarnings := map[string][]string{}

	secs := make([]string, 0, len(deltasBySec))
	for k := range deltasBySec {
		secs = append(secs, k)
	}
	sort.Strings(secs)

	for _, sec := range secs {
		deltas := deltasBySec[sec]
		summaryTxs, warnings := MakeSummaryTxs(latestDate, deltas)
		if warnings != nil {
			// Add warnings to allWarnings
			for _, warning := range warnings {
				var secsWithWarning []string
				var ok bool
				if secsWithWarning, ok = allWarnings[warning]; ok {
					secsWithWarning = append(secsWithWarning, sec)
				} else {
					secsWithWarning = []string{sec}
				}
				allWarnings[warning] = secsWithWarning
			}
		}

		allSummaryTxs = append(allSummaryTxs, summaryTxs...)
	}

	return &CollectedSummaryData{allSummaryTxs, allWarnings, nil}
}
