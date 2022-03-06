package portfolio

import (
	"fmt"
	"time"

	"github.com/tsiemens/acb/date"
)

// Return a slice of Txs which can summarise all txs in `deltas` up to `latestDate`.
// Multiple Txs might be returned if it is not possible to accurately summarise
// in a single Tx without altering superficial losses (and preserving overall
// capital gains?)
//
// Note that `deltas` should provide all TXs for 30 days after latestDate, otherwise
// the summary may become innacurate/problematic if a new TX were added within
// that 30 day period after, and introduce a new superficial loss.
//
// TODO do we want an option to break summaries up so they are per-year? (so we
// also still get a per-year capital gains summary, so we can look backwards?)
//
// TODO NOTE we cannot summarize any TXs within 30 days of any superficial loss
// which is not summarized, since TotalAquiredInPeriod would then be innacurate in
// unsummarized TXs.
//
// eg 1. (cannot be summarized)
// 2021-11-05 BUY  1  @ 1.50
// 2021-12-05 BUY  11 @ 1.50
// 2022-01-01 SELL 10 @ 1.00
//
// Return: summary Txs, user warnings, error
func MakeSummaryTxs(latestDate date.Date, deltas []*TxDelta) ([]*Tx, []string, error) {
	// Step 1: Find the latest Delta <= latestDate
	latestDeltaInSummaryRangeIdx := -1
	for i, delta := range deltas {
		if delta.Tx.Date.After(latestDate) {
			break
		}
		latestDeltaInSummaryRangeIdx = i
	}
	if latestDeltaInSummaryRangeIdx == -1 {
		return nil, nil, fmt.Errorf("No transactions in the summary period")
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

	var summaryPeriodTxs []*Tx

	if latestSummarizableDeltaIdx != -1 {

		tx := deltas[latestSummarizableDeltaIdx].Tx
		// All one TX. No capital gains yet.
		sumPostStatus := deltas[latestSummarizableDeltaIdx].PostStatus
		summaryTx := &Tx{
			Security: tx.Security, Date: tx.Date, Action: BUY,
			Shares:         sumPostStatus.ShareBalance,
			AmountPerShare: sumPostStatus.TotalAcb / float64(sumPostStatus.ShareBalance),
			Commission:     0.0,
			TxCurrency:     DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: 0.0,
			CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: 0.0,
			Memo: "Summary",
		}

		summaryPeriodTxs = append(summaryPeriodTxs, summaryTx)

		// TODO Create a summary Sell TX with the same cap gains for the year?
	}

	for _, delta := range deltas[latestSummarizableDeltaIdx+1 : latestDeltaInSummaryRangeIdx+1] {
		summaryPeriodTxs = append(summaryPeriodTxs, delta.Tx)
	}

	// TODO warnings:
	// - Check the current system date vs latestDate (is it 30 days separated?)
	// - Actually possibly 60 days (or rather, any losses should be more than 30 days ago)
	// - If there is no TX after latestDate, warn they might be missing.

	return summaryPeriodTxs, nil, nil
}
