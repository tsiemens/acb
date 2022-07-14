package portfolio

import (
	"fmt"
	"io"
	"sort"
	"strings"

	tw "github.com/olekukonko/tablewriter"
	"github.com/tsiemens/acb/util"
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

func RenderTxTableModel(deltas []*TxDelta, renderFullDollarValues bool) *RenderTable {
	table := &RenderTable{}
	table.Header = []string{"Security", "Trade Date", "Settl. Date", "TX", "Amount", "Shares", "Amt/Share", "ACB",
		"Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB", "New ACB/Share",
		"Memo",
	}

	ph := _PrintHelper{PrintAllDecimals: renderFullDollarValues}

	var capGainsTotal float64 = 0.0
	capGainsYearTotals := map[int]float64{}
	sawSuperficialLoss := false

	for _, d := range deltas {
		capGainsYearTotals[d.Tx.SettlementDate.Year()] = 0.0
	}

	for _, d := range deltas {
		superficialLossAsterix := ""
		superficialLossAddAsterix := ""
		if d.SuperficialLoss != 0.0 {
			superficialLossAsterix = fmt.Sprintf(" *\n(SFL %s)", ph.PlusMinusDollar(d.SuperficialLoss, false))
			superficialLossAddAsterix = fmt.Sprintf(" *\n(%s)", ph.PlusMinusDollar(-1*d.SuperficialLoss, true))
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
			ph.PlusMinusDollar(d.AcbDelta(), true) + superficialLossAddAsterix,
			"$" + ph.CurrStr(d.PostStatus.TotalAcb) + superficialLossAddAsterix,
			// Acb per share
			strOrDash(d.PostStatus.ShareBalance > 0.0,
				"$"+ph.CurrStr(d.PostStatus.TotalAcb/float64(d.PostStatus.ShareBalance))),
			tx.Memo,
		}
		table.Rows = append(table.Rows, row)

		capGainsTotal += d.CapitalGain
		capGainsYearTotals[tx.SettlementDate.Year()] += d.CapitalGain
	}

	// Footer
	years := util.IntFloat64MapKeys(capGainsYearTotals)
	sort.Ints(years)
	yearStrs := []string{}
	yearValsStrs := []string{}
	for _, year := range years {
		yearStrs = append(yearStrs, fmt.Sprintf("%d", year))
		yearlyTotal := capGainsYearTotals[year]
		yearValsStrs = append(yearValsStrs, ph.PlusMinusDollar(yearlyTotal, false))
	}
	totalFooterLabel := "Total"
	totalFooterValsStr := ph.PlusMinusDollar(capGainsTotal, false)
	if len(years) > 0 {
		totalFooterLabel += "\n" + strings.Join(yearStrs, "\n")
		totalFooterValsStr += "\n" + strings.Join(yearValsStrs, "\n")
	}

	table.Footer = []string{"", "", "", "", "", "", "", "",
		totalFooterLabel, totalFooterValsStr, "", "", "", "", ""}

	// Notes
	if sawSuperficialLoss {
		table.Notes = append(table.Notes, " */SFL = Superficial loss adjustment")
	}

	return table
}

func PrintRenderTable(tableModel *RenderTable, writer io.Writer) {
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
