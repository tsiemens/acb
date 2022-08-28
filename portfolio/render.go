package portfolio

import (
	"fmt"
	"io"
	"strings"

	tw "github.com/olekukonko/tablewriter"
)

type _PrintHelper struct {
	PrintAllDecimals bool
}

func (h _PrintHelper) CurrStr(val float64) string {
	if h.PrintAllDecimals {
		return fmt.Sprintf("%f", val)
	}
	return fmt.Sprintf("%.2f", val)
}

func (h _PrintHelper) CurrWithFxStr(val float64, curr Currency, rateToLocal float64) string {
	if curr == DEFAULT_CURRENCY {
		return "$" + h.CurrStr(val)
	}
	return fmt.Sprintf("$%s\n(%s %s)", h.CurrStr(val*rateToLocal), h.CurrStr(val), curr)
}

func strOrDash(useStr bool, str string) string {
	if useStr {
		return str
	}
	return "-"
}

func (h _PrintHelper) PlusMinusDollar(val float64, showPlus bool) string {
	if val < 0.0 {
		return fmt.Sprintf("-$%s", h.CurrStr(val*-1.0))
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
		"Memo",
	}

	ph := _PrintHelper{PrintAllDecimals: renderFullDollarValues}

	sawSuperficialLoss := false

	for _, d := range deltas {
		superficialLossAsterix := ""
		if d.SuperficialLoss != 0.0 {
			superficialLossAsterix = fmt.Sprintf(
				" *\n(SfL %s; %d/%d)",
				ph.PlusMinusDollar(d.SuperficialLoss, false),
				d.SuperficialLossRatio.Numerator,
				d.SuperficialLossRatio.Denominator,
			)
			sawSuperficialLoss = true
		}
		tx := d.Tx

		var preAcbPerShare float64 = 0.0
		if tx.Action == SELL && d.PreStatus.ShareBalance > 0 {
			preAcbPerShare = d.PreStatus.TotalAcb / float64(d.PreStatus.ShareBalance)
		}

		row := []string{d.Tx.Security, tx.TradeDate.String(), tx.SettlementDate.String(), tx.Action.String(),
			// Amount
			ph.CurrWithFxStr(float64(tx.Shares)*tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			fmt.Sprintf("%d", tx.Shares),
			ph.CurrWithFxStr(tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			// ACB of sale
			strOrDash(tx.Action == SELL, "$"+ph.CurrStr(preAcbPerShare*float64(tx.Shares))),
			// Commission
			strOrDash(tx.Commission != 0.0,
				ph.CurrWithFxStr(tx.Commission, tx.CommissionCurrency, tx.CommissionCurrToLocalExchangeRate)),
			// Cap gains
			strOrDash(tx.Action == SELL, ph.PlusMinusDollar(d.CapitalGain, false)+superficialLossAsterix),
			fmt.Sprintf("%d", d.PostStatus.ShareBalance),
			ph.PlusMinusDollar(d.AcbDelta(), true),
			"$" + ph.CurrStr(d.PostStatus.TotalAcb),
			// Acb per share
			strOrDash(d.PostStatus.ShareBalance > 0.0,
				"$"+ph.CurrStr(d.PostStatus.TotalAcb/float64(d.PostStatus.ShareBalance))),
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
		totalFooterLabel, totalFooterValsStr, "", "", "", "", ""}

	// Notes
	if sawSuperficialLoss {
		table.Notes = append(table.Notes, " */SfL = Superficial loss adjustment")
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
