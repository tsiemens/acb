package portfolio

import (
	"fmt"
	"io"
	"os"

	tw "github.com/olekukonko/tablewriter"
	"github.com/tsiemens/acb/util"
)

func currStr(val float64) string {
	return fmt.Sprintf("%.2f", val)
}

func currWithFxStr(val float64, curr Currency, rateToLocal float64) string {
	if curr == DEFAULT_CURRENCY {
		return "$" + currStr(val)
	}
	return fmt.Sprintf("$%s\n(%s %s)", currStr(val*rateToLocal), currStr(val), curr)
}

func strOrDash(useStr bool, str string) string {
	if useStr {
		return str
	}
	return "-"
}

func plusMinusDollar(val float64, showPlus bool) string {
	if val < 0.0 {
		return fmt.Sprintf("-$%s", currStr(val*-1.0))
	}
	plus := ""
	if showPlus {
		plus = "+"
	}
	return fmt.Sprintf("%s$%s", plus, currStr(val))
}

type RenderTable struct {
	Header []string
	Rows   [][]string
	Footer []string
	Notes  []string
	Errors []error
}

func RenderTxTableModel(deltas []*TxDelta) *RenderTable {
	table := &RenderTable{}
	table.Header = []string{"Security", "Date", "TX", "Amount", "Shares", "Amt/Share", "ACB",
		"Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB", "New ACB/Share",
		"Memo",
	}

	var capGainsTotal float64 = 0.0
	sawSuperficialLoss := false

	for _, d := range deltas {
		superficialLossAsterix := ""
		superficialLossAddAsterix := ""
		if d.SuperficialLoss != 0.0 {
			superficialLossAsterix = fmt.Sprintf(" *\n(was %s)", plusMinusDollar(d.SuperficialLoss, false))
			superficialLossAddAsterix = fmt.Sprintf(" *\n(%s)", plusMinusDollar(-1*d.SuperficialLoss, true))
			sawSuperficialLoss = true
		}
		tx := d.Tx

		var preAcbPerShare float64 = 0.0
		if tx.Action == SELL && d.PreStatus.ShareBalance > 0 {
			preAcbPerShare = d.PreStatus.TotalAcb / float64(d.PreStatus.ShareBalance)
		}

		row := []string{d.Tx.Security, util.DateStr(tx.Date), tx.Action.String(),
			// Amount
			currWithFxStr(float64(tx.Shares)*tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			fmt.Sprintf("%d", tx.Shares),
			currWithFxStr(tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			// ACB of sale
			strOrDash(tx.Action == SELL, "$"+currStr(preAcbPerShare*float64(tx.Shares))),
			// Commission
			strOrDash(tx.Commission != 0.0,
				currWithFxStr(tx.Commission, tx.CommissionCurrency, tx.CommissionCurrToLocalExchangeRate)),
			// Cap gains
			strOrDash(tx.Action == SELL, plusMinusDollar(d.CapitalGain, false)+superficialLossAsterix),
			fmt.Sprintf("%d", d.PostStatus.ShareBalance),
			plusMinusDollar(d.AcbDelta(), true) + superficialLossAddAsterix,
			"$" + currStr(d.PostStatus.TotalAcb) + superficialLossAddAsterix,
			// Acb per share
			strOrDash(d.PostStatus.ShareBalance > 0.0,
				"$"+currStr(d.PostStatus.TotalAcb/float64(d.PostStatus.ShareBalance))),
			tx.Memo,
		}
		table.Rows = append(table.Rows, row)

		capGainsTotal += d.CapitalGain
	}
	table.Footer = []string{"", "", "", "", "", "", "",
		"Total", plusMinusDollar(capGainsTotal, false), "", "", "", "", ""}

	if sawSuperficialLoss {
		table.Notes = append(table.Notes, " * = Superficial loss adjustment")
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

func RenderTxTable(deltas []*TxDelta) {
	tableModel := RenderTxTableModel(deltas)
	PrintRenderTable(tableModel, os.Stdout)
}
