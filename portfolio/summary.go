package portfolio

import (
	"fmt"
	"os"
	"sort"
	"time"

	"github.com/tsiemens/acb/date"
	decimal "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/log"
	"github.com/tsiemens/acb/util"
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
// eg 1. (cannot be summarized)
// 2021-11-05 BUY  1  @ 1.50
// 2021-12-05 BUY  11 @ 1.50
// 2022-01-01 SELL 10 @ 1.00
//
// Return: summary Txs, user warnings, error
func MakeSummaryTxs(latestDate date.Date, deltas []*TxDelta, splitAnnualGains bool) ([]*Tx, []string) {
	latestDeltaInSummaryRangeIdx, latestSummarizableDeltaIdx, warnings_ :=
		getSummaryRangeDeltaIndicies(latestDate, deltas)
	if latestDeltaInSummaryRangeIdx < 0 {
		return nil, warnings_
	}

	// Create a map of affiliate to its last summarizable delta index. Not all
	// affiliates will have one.
	affilLastSummarizableDeltaIdxs := map[*Affiliate]int{}
	// affilIds will be sorted alphabetically for determinism
	affilIds := []string{}
	for i := latestSummarizableDeltaIdx; i >= 0; i-- {
		af := NonNilTxAffiliate(deltas[i].Tx)
		if _, ok := affilLastSummarizableDeltaIdxs[af]; !ok {
			affilLastSummarizableDeltaIdxs[af] = i
			affilIds = append(affilIds, af.Id())
		}
	}
	sort.Strings(affilIds)

	var summaryPeriodTxs []*Tx = make([]*Tx, 0, 0)
	warnings := util.NewSet[string]()
	for _, afId := range affilIds {
		af := GlobalAffiliateDedupTable.MustGet(afId)
		affilLastSummarizableDeltaIdx := affilLastSummarizableDeltaIdxs[af]

		var afSumTxs []*Tx
		var warns []string
		if splitAnnualGains {
			afSumTxs, warns = makeAnnualGainsSummaryTxs(
				af, deltas, affilLastSummarizableDeltaIdx)
		} else {
			afSumTxs, warns = makeSimpleSummaryTxs(
				af, deltas, affilLastSummarizableDeltaIdx)
		}
		summaryPeriodTxs = append(summaryPeriodTxs, afSumTxs...)
		warnings.AddAll(warns)
	}
	for i, tx := range summaryPeriodTxs {
		tx.ReadIndex = uint32(i)
	}
	summaryPeriodTxs = SortTxs(summaryPeriodTxs)
	for _, tx := range summaryPeriodTxs {
		tx.ReadIndex = 0
	}

	unsummarizableTxs := deltas[latestSummarizableDeltaIdx+1 : latestDeltaInSummaryRangeIdx+1]
	if len(unsummarizableTxs) > 0 {
		warn := fmt.Sprintf(
			"Some transactions to be summarized (between %s and %s) could not be due to superficial-loss conflicts",
			unsummarizableTxs[0].Tx.TradeDate.String(),
			unsummarizableTxs[len(unsummarizableTxs)-1].Tx.TradeDate.String(),
		)
		warnings.Add(warn)
	}
	for _, delta := range unsummarizableTxs {
		unsumTx := delta.Tx
		if delta.IsSuperficialLoss() {
			// The proess of generating the deltas will create Sfla Tx/Deltas, so
			// if we emit these as a csv, we MUST convert all SFL sales to have
			// explicit superficial losses.
			// Copy, and add add SFL
			unsumTx = &*delta.Tx
			unsumTx.SpecifiedSuperficialLoss = util.NewOptional[SFLInput](
				SFLInput{delta.SuperficialLoss, false /* force */})
		}
		summaryPeriodTxs = append(summaryPeriodTxs, unsumTx)
	}

	today := date.Today()
	if latestDeltaInSummaryRangeIdx != -1 {
		lastSummarizableDelta := deltas[latestDeltaInSummaryRangeIdx]
		// Find the very latest day that could possibly ever affect or be affected by
		// the last tx. This should be 60 days.
		lastAffectingDay := GetLastDayInSuperficialLossPeriod(
			GetLastDayInSuperficialLossPeriod(lastSummarizableDelta.Tx.SettlementDate))
		if !today.After(lastAffectingDay) {
			warnings.Add(
				"The current date is such that new TXs could potentially alter how the " +
					"summary is created. You should wait 60 days after your latest " +
					"transaction within the summary period to generate the summary")
		}
	}

	var warningsSlice []string
	if warnings.Len() > 0 {
		warningsSlice = warnings.ToSlice()
	}
	return summaryPeriodTxs, warningsSlice
}

// Returns: latestDeltaInSummaryRangeIdx, latestSummarizableDeltaIdx, warnings
func getSummaryRangeDeltaIndicies(latestDate date.Date, deltas []*TxDelta) (int, int, []string) {
	// Step 1: Find the latest Delta <= latestDate
	latestDeltaInSummaryRangeIdx := -1
	for i, delta := range deltas {
		if delta.Tx.SettlementDate.After(latestDate) {
			break
		}
		latestDeltaInSummaryRangeIdx = i
	}
	if latestDeltaInSummaryRangeIdx == -1 {
		return -1, -1, []string{"No transactions in the summary period"}
	}

	// Step 2: determine if any of the TXs within 30 days of latestDate are
	// superficial losses.
	// If any are, save the date 30 days prior to it (firstSuperficialLossPeriodDay)
	txInSummaryOverlapsSuperficialLoss := false
	firstSuperficialLossPeriodDay := date.New(3000, time.January, 1)
	latestInSummaryTx := deltas[latestDeltaInSummaryRangeIdx].Tx
	latestInSummaryDate := latestInSummaryTx.SettlementDate
	for _, delta := range deltas[latestDeltaInSummaryRangeIdx+1:] {
		if delta.IsSuperficialLoss() {
			firstSuperficialLossPeriodDay = GetFirstDayInSuperficialLossPeriod(delta.Tx.SettlementDate)
			txInSummaryOverlapsSuperficialLoss = !latestInSummaryDate.Before(firstSuperficialLossPeriodDay)
			if txInSummaryOverlapsSuperficialLoss {
				log.Fverbosef(os.Stderr,
					"getSummaryRangeDeltaIndicies: %s tx in %s settled on %s is in SFL period "+
						"(starting %s) of tx settled on %s (SFL of %f)\n",
					latestInSummaryTx.Security, latestInSummaryTx.Affiliate.Name(),
					latestInSummaryTx.SettlementDate, firstSuperficialLossPeriodDay.String(),
					delta.Tx.SettlementDate, delta.SuperficialLoss,
				)
			}
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
		var latestSummarizableDate date.Date
		for i := latestDeltaInSummaryRangeIdx; i >= 0; i-- {
			delta := deltas[i]
			if delta.Tx.SettlementDate.Before(firstSuperficialLossPeriodDay) {
				latestSummarizableDeltaIdx = i
				latestSummarizableDate = delta.Tx.SettlementDate
				break
			}
			if delta.IsSuperficialLoss() {
				// We've encountered another superficial loss within the summary
				// range. This can be affected by previous txs, so we need to now push
				// up the period where we can't find any txs.
				firstSuperficialLossPeriodDay = GetFirstDayInSuperficialLossPeriod(delta.Tx.SettlementDate)
			}
		}
		log.Fverbosef(os.Stderr, "   latestSummarizableDeltaIdx: %d (%s)\n",
			latestSummarizableDeltaIdx, latestSummarizableDate.String())
	} else {
		latestSummarizableDeltaIdx = latestDeltaInSummaryRangeIdx
	}

	return latestDeltaInSummaryRangeIdx, latestSummarizableDeltaIdx, nil
}

const shareBalanceZeroWarning = "Share balance at the end of the summarized period was zero"

func makeSimpleSummaryTxs(
	af *Affiliate, deltas []*TxDelta, latestSummarizableDeltaIdx int) ([]*Tx, []string) {

	var warnings []string
	summaryPeriodTxs := []*Tx{}

	if latestSummarizableDeltaIdx != -1 {

		tx := deltas[latestSummarizableDeltaIdx].Tx
		// All one TX. No capital gains yet.
		sumPostStatus := deltas[latestSummarizableDeltaIdx].PostStatus
		if !sumPostStatus.ShareBalance.IsZero() {
			summaryTx := &Tx{
				Security: tx.Security,
				// Use same day for TradeDate and SettlementDate, since this is not
				// a real trade, and has no exchange rate to depend on the trade date.
				TradeDate:      tx.SettlementDate,
				SettlementDate: tx.SettlementDate,
				Action:         BUY,
				Shares:         sumPostStatus.ShareBalance,
				AmountPerShare: sumPostStatus.TotalAcb.Div(sumPostStatus.ShareBalance),
				Commission:     decimal.Zero,
				TxCurrency:     DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: decimal.Zero,
				CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: decimal.Zero,
				Memo:      "Summary",
				Affiliate: af,
				ReadIndex: 0, // This needs to be the first Tx in the list.
			}

			summaryPeriodTxs = append(summaryPeriodTxs, summaryTx)
		} else {
			warnings = append(warnings, shareBalanceZeroWarning)
		}
	}

	return summaryPeriodTxs, warnings
}

func makeAnnualGainsSummaryTxs(
	af *Affiliate, deltas []*TxDelta, latestSummarizableDeltaIdx int) ([]*Tx, []string) {

	var warnings []string
	summaryPeriodTxs := []*Tx{}

	if latestSummarizableDeltaIdx == -1 {
		return summaryPeriodTxs, warnings
	}

	yearlyCapGains := map[int]decimal.Decimal{}
	latestYearDelta := map[int]*TxDelta{}
	firstYear := deltas[0].Tx.SettlementDate.Year()
	if !af.Registered() {
		for _, delta := range deltas[:latestSummarizableDeltaIdx+1] {
			if NonNilTxAffiliate(delta.Tx) != af {
				continue
			}
			year := delta.Tx.SettlementDate.Year()
			if !delta.CapitalGain.IsZero() {
				if gain, ok := yearlyCapGains[year]; ok {
					yearlyCapGains[year] = gain.Add(delta.CapitalGain)
				} else {
					yearlyCapGains[year] = delta.CapitalGain
				}
			}
			latestYearDelta[year] = delta
		}
	}

	yearsWithGains := make([]int, 0, len(yearlyCapGains))
	for year, _ := range yearlyCapGains {
		yearsWithGains = append(yearsWithGains, year)
	}
	sort.Ints(yearsWithGains)

	readIndex := uint32(0)

	sumPostStatus := deltas[latestSummarizableDeltaIdx].PostStatus
	baseAcbPerShare := decimal.Zero
	if !sumPostStatus.ShareBalance.IsZero() {
		baseAcbPerShare = sumPostStatus.TotalAcb.Div(sumPostStatus.ShareBalance)
	}

	if sumPostStatus.ShareBalance.IsZero() {
		warnings = append(warnings, shareBalanceZeroWarning)
	}

	// Add length of yearsWithGains to the share balance, as we'll sell one share per year
	// This will generally always be non-zero for non-registered affiliates
	nBaseShares := sumPostStatus.ShareBalance.Add(decimal.NewFromInt(int64(len(yearsWithGains))))
	if nBaseShares.IsPositive() {
		tx := deltas[latestSummarizableDeltaIdx].Tx
		// Get the earliest year, and use Jan 1 of the previous year for the buy.
		dt := date.New(uint32(firstYear-1), time.January, 1)
		setupBuySumTx := &Tx{
			Security:       tx.Security,
			TradeDate:      dt,
			SettlementDate: dt,
			Action:         BUY,
			Shares:         nBaseShares,
			AmountPerShare: baseAcbPerShare,
			Commission:     decimal.Zero,
			TxCurrency:     DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: decimal.Zero,
			CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: decimal.Zero,
			Memo:      "Summary base (buy)",
			Affiliate: af,
			ReadIndex: readIndex,
		}
		summaryPeriodTxs = append(summaryPeriodTxs, setupBuySumTx)
	}

	for _, year := range yearsWithGains {
		gain := yearlyCapGains[year]
		loss := decimal.Zero
		if gain.IsNegative() {
			loss = gain.Neg()
			gain = decimal.Zero
		}
		tx := latestYearDelta[year].Tx
		dt := date.New(uint32(tx.SettlementDate.Year()), time.January, 1)
		summaryTx := &Tx{
			Security:       tx.Security,
			TradeDate:      dt,
			SettlementDate: dt,
			Action:         SELL,
			Shares:         decimal.NewFromInt(1),
			AmountPerShare: baseAcbPerShare.Add(gain),
			Commission:     loss,
			TxCurrency:     DEFAULT_CURRENCY, TxCurrToLocalExchangeRate: decimal.Zero,
			CommissionCurrency: DEFAULT_CURRENCY, CommissionCurrToLocalExchangeRate: decimal.Zero,
			Memo:      fmt.Sprintf("%d gain summary (sell)", year),
			Affiliate: af,
			ReadIndex: readIndex, // This needs to be before the Buy
		}
		summaryPeriodTxs = append(summaryPeriodTxs, summaryTx)
	}

	return summaryPeriodTxs, warnings
}

// TODO summarize annually generally. ie, have the amount bought and sold each year
// be accurate, as well as the gains/loss.

type CollectedSummaryData struct {
	Txs []*Tx
	// Warnings -> list of secs that encountered this warning
	Warnings map[string][]string
	// Security -> errors encountered (populated externally)
	Errors map[string][]error
}

func MakeAggregateSummaryTxs(
	latestDate date.Date,
	deltasBySec map[string][]*TxDelta,
	splitAnnualGains bool) *CollectedSummaryData {

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
		summaryTxs, warnings := MakeSummaryTxs(latestDate, deltas, splitAnnualGains)
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
