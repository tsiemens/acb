package portfolio

import (
	"fmt"
	"io"
	"os"
	"strings"

	tw "github.com/olekukonko/tablewriter"

	decimal "github.com/tsiemens/acb/decimal_value"
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
	return val.Decimal.StringFixed(2)
}

func (h _PrintHelper) DollarStr(val decimal.Decimal) string {
	if val.IsNull {
		return NaNString()
	}
	return "$" + h.CurrStr(val)
}

func (h _PrintHelper) CurrWithFxStr(val decimal.Decimal, curr Currency, rateToLocal decimal.Decimal) string {
	if curr == DEFAULT_CURRENCY {
		return h.DollarStr(val)
	}
	return fmt.Sprintf("%s\n(%s %s)", h.DollarStr(val.Mul(rateToLocal)), h.CurrStr(val), curr)
}

func strOrDash(useStr bool, str string) string {
	if useStr {
		return str
	}
	return "-"
}

func (h _PrintHelper) PlusMinusDollar(val decimal.Decimal, showPlus bool) string {
	if val.IsNull {
		return NaNString()
	}
	if val.IsNegative() {
		return fmt.Sprintf("-$%s", h.CurrStr(val.Mul(decimal.NewFromInt(-1))))
	}
	plus := ""
	if showPlus {
		plus = "+"
	}
	return fmt.Sprintf("%s$%s", plus, h.CurrStr(val))
}

type RenderTable struct {
	Header []string
	Rows   [][]string
	Footer []string
	Notes  []string
	Errors []error
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
				" *\n(SfL %s%s; %d/%d%s)",
				ph.PlusMinusDollar(d.SuperficialLoss, false),
				util.Tern[string](specifiedSflIsForced, "!", ""),
				d.SuperficialLossRatio.Numerator,
				d.SuperficialLossRatio.Denominator,
				extraSflNoteStr,
			)
			sawSuperficialLoss = true
			sawOverAppliedSfl = sawOverAppliedSfl || d.PotentiallyOverAppliedSfl
		}
		tx := d.Tx

		var preAcbPerShare decimal.Decimal
		if tx.Action == SELL && d.PreStatus.ShareBalance.IsPositive() {
			preAcbPerShare = d.PreStatus.TotalAcb.Div(d.PreStatus.ShareBalance)
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
			strOrDash(tx.Action == SELL, ph.DollarStr(preAcbPerShare.Mul(tx.Shares))),
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
				ph.DollarStr(d.PostStatus.TotalAcb.Div(d.PostStatus.ShareBalance))),
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

/*
  Generates a RenderTable that will render out to this:
  | Year             | Capital Gains |
  +------------------+---------------+
  | 2000             | xxxx.xx       |
  | 2001             | xxxx.xx       |
  | Since inception  | xxxx.xx       |
*/
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

func PrintRenderTable(title string, tableModel *RenderTable, writer io.Writer) {
	for _, err := range tableModel.Errors {
		fmt.Fprintf(writer, "[!] %v. Printing parsed information state:\n", err)
	}
	fmt.Fprintf(writer, "%s\n", title)

	table := tw.NewWriter(writer)
	table.SetHeader(tableModel.Header)
	table.SetBorder(false)
	table.SetRowLine(true)

	for _, row := range tableModel.Rows {
		table.Append(row)
	}

	table.SetFooter(tableModel.Footer)

	table.Render()

	for _, note := range tableModel.Notes {
		fmt.Fprintln(writer, note)
	}
}
