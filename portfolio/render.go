package portfolio

import (
	"fmt"
	"os"
	"sort"
	"strings"

	"github.com/shopspring/decimal"

	"github.com/tsiemens/acb/date"
	decimal_opt "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/util"
)

type _PrintHelper struct {
	PrintAllDecimals bool
}

var displayNanEnvSetting util.Optional[string]

func NaNString() string {
	if !displayNanEnvSetting.Present() {
		displayNanEnvSetting.Set(os.Getenv("DISPLAY_NAN"))
	}
	if displayNanEnvSetting.MustGet() == "" || displayNanEnvSetting.MustGet() == "0" {
		return "-"
	}
	return "NaN"
}

func (h _PrintHelper) CurrStr(val decimal.Decimal) string {
	if h.PrintAllDecimals {
		return val.String()
	}
	return val.StringFixed(2)
}

func (h _PrintHelper) OptCurrStr(val decimal_opt.DecimalOpt) string {
	if val.IsNull {
		return val.String()
	}
	return h.CurrStr(val.Decimal)
}

func (h _PrintHelper) DollarStr(val decimal_opt.DecimalOpt) string {
	if val.IsNull {
		return NaNString()
	}
	return "$" + h.OptCurrStr(val)
}

func (h _PrintHelper) CurrWithFxStr(val decimal.Decimal, curr Currency, rateToLocal decimal.Decimal) string {
	if curr == DEFAULT_CURRENCY {
		return h.DollarStr(decimal_opt.New(val))
	}
	return fmt.Sprintf("%s\n(%s %s)", h.DollarStr(decimal_opt.New(val.Mul(rateToLocal))), h.CurrStr(val), curr)
}

func strOrDash(useStr bool, str string) string {
	if useStr {
		return str
	}
	return "-"
}

func (h _PrintHelper) PlusMinusDollar(val decimal_opt.DecimalOpt, showPlus bool) string {
	if val.IsNull {
		return NaNString()
	}
	if val.IsNegative() {
		return fmt.Sprintf("-$%s", h.OptCurrStr(val.Neg()))
	}
	plus := ""
	if showPlus {
		plus = "+"
	}
	return fmt.Sprintf("%s$%s", plus, h.OptCurrStr(val))
}

type RenderTable struct {
	Header []string
	Rows   [][]string
	Footer []string
	Notes  []string
	Errors []error
}

type CostsTables struct {
	Total  *RenderTable
	Yearly *RenderTable
}

func RenderTxTableModel(
	deltas []*TxDelta, gains *CumulativeCapitalGains, renderFullDollarValues bool) *RenderTable {
	table := &RenderTable{}
	table.Header = []string{"Security", "Trade Date", "Settl. Date", "TX", "Amount", "Shares", "Amt/Share", "ACB",
		"Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB", "New ACB/Share",
		"Affiliate", "Memo",
	}

	ph := _PrintHelper{PrintAllDecimals: renderFullDollarValues}

	sawSuperficialLoss := false
	sawOverAppliedSfl := false

	for _, d := range deltas {
		superficialLossAsterix := ""
		specifiedSflIsForced := d.Tx.SpecifiedSuperficialLoss.Present() &&
			d.Tx.SpecifiedSuperficialLoss.MustGet().Force
		if d.IsSuperficialLoss() {
			extraSflNoteStr := ""
			if d.PotentiallyOverAppliedSfl {
				extraSflNoteStr = " [1]"
			}

			superficialLossAsterix = fmt.Sprintf(
				" *\n(SfL %s%s; %s/%s%s)",
				ph.PlusMinusDollar(d.SuperficialLoss, false),
				util.Tern(specifiedSflIsForced, "!", ""),
				d.SuperficialLossRatio.Numerator,
				d.SuperficialLossRatio.Denominator,
				extraSflNoteStr,
			)
			sawSuperficialLoss = true
			sawOverAppliedSfl = sawOverAppliedSfl || d.PotentiallyOverAppliedSfl
		}
		tx := d.Tx

		var preAcbPerShare decimal_opt.DecimalOpt
		if tx.Action == SELL && d.PreStatus.ShareBalance.IsPositive() {
			preAcbPerShare = d.PreStatus.TotalAcb.DivD(d.PreStatus.ShareBalance)
		}

		var affiliateName string
		if tx.Affiliate != nil {
			affiliateName = tx.Affiliate.Name()
		} else {
			affiliateName = GlobalAffiliateDedupTable.GetDefaultAffiliate().Name()
		}

		row := []string{d.Tx.Security, tx.TradeDate.String(), tx.SettlementDate.String(), tx.Action.String(),
			// Amount
			ph.CurrWithFxStr(tx.Shares.Mul(tx.AmountPerShare), tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			tx.Shares.String(),
			ph.CurrWithFxStr(tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			// ACB of sale
			strOrDash(tx.Action == SELL, ph.DollarStr(preAcbPerShare.MulD(tx.Shares))),
			// Commission
			strOrDash(!tx.Commission.IsZero(),
				ph.CurrWithFxStr(tx.Commission, tx.CommissionCurrency, tx.CommissionCurrToLocalExchangeRate)),
			// Cap gains
			strOrDash(tx.Action == SELL, ph.PlusMinusDollar(d.CapitalGain, false)+superficialLossAsterix),
			util.Tern(d.PostStatus.ShareBalance.Equal(d.PostStatus.AllAffiliatesShareBalance),
				d.PostStatus.ShareBalance.String(),
				fmt.Sprintf("%s / %s", d.PostStatus.ShareBalance, d.PostStatus.AllAffiliatesShareBalance)),
			ph.PlusMinusDollar(d.AcbDelta(), true),
			ph.DollarStr(d.PostStatus.TotalAcb),
			// Acb per share
			strOrDash(d.PostStatus.ShareBalance.IsPositive(),
				ph.DollarStr(d.PostStatus.TotalAcb.DivD(d.PostStatus.ShareBalance))),
			affiliateName,
			tx.Memo,
		}
		table.Rows = append(table.Rows, row)
	}

	// Footer
	years := gains.CapitalGainsYearTotalsKeysSorted()
	yearStrs := []string{}
	yearValsStrs := []string{}
	for _, year := range years {
		yearStrs = append(yearStrs, fmt.Sprintf("%d", year))
		yearlyTotal := gains.CapitalGainsYearTotals[year]
		yearValsStrs = append(yearValsStrs, ph.PlusMinusDollar(yearlyTotal, false))
	}
	totalFooterLabel := "Total"
	totalFooterValsStr := ph.PlusMinusDollar(gains.CapitalGainsTotal, false)
	if len(years) > 0 {
		totalFooterLabel += "\n" + strings.Join(yearStrs, "\n")
		totalFooterValsStr += "\n" + strings.Join(yearValsStrs, "\n")
	}

	table.Footer = []string{"", "", "", "", "", "", "", "",
		totalFooterLabel, totalFooterValsStr, "", "", "", "", "", ""}

	// Notes
	if sawSuperficialLoss {
		table.Notes = append(table.Notes, " SfL = Superficial loss adjustment")
	}
	if sawOverAppliedSfl {
		table.Notes = append(table.Notes,
			" [1] Superficial loss was potentially over-applied, resulting in a lower-than-expected allowable capital loss.\n"+
				"     See I.1 vs I.2 under \"Interpretations of ACB distribution\" at https://github.com/tsiemens/acb/wiki/Superficial-Losses")
	}

	return table
}

// RenderAggregateCapitalGains generates a RenderTable that will render out to this:
//
//	| Year             | Capital Gains |
//	+------------------+---------------+
//	| 2000             | xxxx.xx       |
//	| 2001             | xxxx.xx       |
//	| Since inception  | xxxx.xx       |
func RenderAggregateCapitalGains(
	gains *CumulativeCapitalGains, renderFullDollarValues bool) *RenderTable {

	table := &RenderTable{}
	table.Header = []string{"Year", "Capital Gains"}

	ph := _PrintHelper{PrintAllDecimals: renderFullDollarValues}

	years := gains.CapitalGainsYearTotalsKeysSorted()
	for _, year := range years {
		yearlyTotal := gains.CapitalGainsYearTotals[year]
		table.Rows = append(
			table.Rows,
			[]string{fmt.Sprintf("%d", year), ph.PlusMinusDollar(yearlyTotal, false)})
	}
	table.Rows = append(
		table.Rows,
		[]string{"Since inception", ph.PlusMinusDollar(gains.CapitalGainsTotal, false)})

	return table
}

// yearInfo tracks the metadata associated with a yearly maximum "total costs" value.
type yearInfo struct {
	day    date.Date              // the date at which the yearly max value was seen
	total  decimal_opt.DecimalOpt // the total ACB of all securities on that date
	values []string               // the securities values for the related row from "total costs"
}

// RenderTotalCosts generates two RenderTable instances in a CostsTable to determine the maximum
// total cost of all included securities at any point during the year, and then to find the maximum
// value of each year.
//
// We do this by looking at any date with one or more TxDelta settlements. We take the maximum for the
// day (in the case of multiple settlements on a given day for the same security), and then add that to
// the current ACB for all the other tracked securities on that day. If there isn't a TxDelta settlement
// for any of the securities on a given day, we use the last value seen. For this reason we must process
// all the TxDeltas in date order.
//
// Examples:
//
//	Total:
//
//		     DATE    |  TOTAL  |  SECA   |  XXXX
//		-------------+---------+---------+---------
//		  2001-01-13 | $100.00 | $100.00 | $0.00
//		-------------+---------+---------+---------
//		  2001-02-14 | $190.00 | $100.00 | $90.00
//		-------------+---------+---------+---------
//		  2001-03-15 | $90.00  | $0.00   | $90.00
//		-------------+---------+---------+---------
//		  2001-04-16 | $80.00  | $0.00   | $80.00
//		-------------+---------+---------+---------
//		  2001-05-17 | $270.00 | $200.00 | $70.00
//		-------------+---------+---------+---------
//		  2003-01-01 | $70.00  | $0.00   | $70.00
//		-------------+---------+---------+---------
//
//
//	Yearly:
//
//		  YEAR |    DATE    |  TOTAL  |  SECA   |  XXXX
//		-------+------------+---------+---------+---------
//		  2001 | 2001-05-17 | $270.00 | $200.00 | $70.00
//		-------+------------+---------+---------+---------
//		  2003 | 2003-01-01 | $70.00  | $0.00   | $70.00
//		-------+------------+---------+---------+---------
func RenderTotalCosts(allDeltas []*TxDelta, renderFullDollarValues bool) *CostsTables {
	dateCosts := map[date.Date]*util.DefaultMap[string, decimal_opt.DecimalOpt]{}

	// For the rendered tables, we need all of the security tickers / names.
	securitySet := map[string]struct{}{}

	// Keep track of the maximum cost for each security on any date where there's a TxDelta.
	// For example, SECA on 2000-01-01 has ACB 12, ACB 150, and ACB 0, so after the loop below,
	// we'll have a dateCosts[2001-01-01][SECA] = 150
	for _, d := range allDeltas {
		dateFromDelta := d.Tx.SettlementDate
		if _, ok := dateCosts[dateFromDelta]; !ok {
			dateCosts[dateFromDelta] = util.NewDefaultMap(func(string) decimal_opt.DecimalOpt {
				return decimal_opt.Null
			})
		}
		sec := d.PostStatus.Security
		securitySet[sec] = struct{}{}

		val := dateCosts[dateFromDelta].Get(sec)
		if val.IsNull {
			val = decimal_opt.Zero
		}
		curMax := decimal_opt.Max(val, d.PostStatus.TotalAcb)
		dateCosts[dateFromDelta].Set(sec, curMax)
	}

	// The sorted set of security names from securitySet
	securities := make([]string, 0, len(securitySet))
	for k := range securitySet {
		securities = append(securities, k)
	}
	sort.Strings(securities)

	// Create the "Total Costs" table (one entry per TxDelta settlement date).
	tcHdr := []string{"Date", "Total"}
	tcHdr = append(tcHdr, securities...)
	totalCost := &RenderTable{Header: tcHdr}

	ph := _PrintHelper{PrintAllDecimals: renderFullDollarValues}

	// The most recently seen ACB for each security. We assume zero as the starting point.
	lastACB := map[string]decimal.Decimal{}

	// The maximum value in a calendar year
	yearMax := map[int]*yearInfo{}

	// We need to process all of the costs in date order so that our lastACB value is sensible.
	sortedDays := make([]date.Date, 0, len(dateCosts))
	for d := range dateCosts {
		sortedDays = append(sortedDays, d)
	}
	sort.Slice(sortedDays, func(i, j int) bool { return sortedDays[i].Before(sortedDays[j]) })

	for _, day := range sortedDays {
		dayTotal := decimal_opt.Zero
		secvals := make([]string, 0, len(securities))
		for _, sec := range securities {
			cur := dateCosts[day].Get(sec)
			var secVal decimal_opt.DecimalOpt
			if cur.IsNull {
				// If we don't have a value for the security on "day", use the most recent one, instead.
				secVal = decimal_opt.New(lastACB[sec])
			} else {
				lastACB[sec] = cur.Decimal
				secVal = cur
			}
			dayTotal = dayTotal.Add(secVal)
			secvals = append(secvals, ph.DollarStr(secVal))
		}
		row := []string{day.String(), ph.DollarStr(dayTotal)}
		row = append(row, secvals...)
		totalCost.Rows = append(totalCost.Rows, row)

		// Check if today's value is the max for the year. If we don't have an entry yet for the year, then
		// start a new one.
		year := day.Year()
		if ym, ok := yearMax[year]; !ok || dayTotal.GreaterThan(ym.total) {
			info := &yearInfo{
				day:    day,
				total:  dayTotal,
				values: secvals,
			}
			yearMax[year] = info
		}
	}

	// Create the "Yearly Max Total Costs" table, one entry per year.
	years := make([]int, 0, len(yearMax))
	for k := range yearMax {
		years = append(years, k)
	}
	sort.Ints(years)

	yrHdr := []string{"Year", "Date", "Total"}
	yrHdr = append(yrHdr, securities...)
	yearCost := &RenderTable{Header: yrHdr}
	for _, year := range years {
		info := yearMax[year]
		row := []string{fmt.Sprint(year), info.day.String()}
		row = append(row, ph.DollarStr(info.total))
		row = append(row, info.values...)
		yearCost.Rows = append(yearCost.Rows, row)
	}

	return &CostsTables{Total: totalCost, Yearly: yearCost}
}
