package portfolio

import (
	"fmt"
	"os"
	"time"

	tw "github.com/olekukonko/tablewriter"
	//"github.com/tsiemens/acb/util"
)

func dateStr(date time.Time) string {
	year, month, day := date.Date()
	return fmt.Sprintf("%d-%02d-%02d", year, month, day)
}

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

func RenderTxTableTest() {
	sptf := &PortfolioSecurityStatus{Security: "FOO", ShareBalance: 0, TotalAcb: 0.0}
	tx_ := &Tx{Security: "FOO", Date: time.Now(), Action: BUY,
		Shares: 2, AmountPerShare: 13.0, Commission: 1.0,
		TxCurrency: CAD, TxCurrToLocalExchangeRate: 1.0,
		CommissionCurrency: CAD, CommissionCurrToLocalExchangeRate: 1.0}

	delta1, err := AddTx(tx_, sptf)
	if err != nil {
		return
	}

	tx_ = &Tx{Security: "FOO", Date: time.Now(), Action: SELL,
		Shares: 1, AmountPerShare: 14.0, Commission: 1.0,
		TxCurrency: USD, TxCurrToLocalExchangeRate: 1.2,
		CommissionCurrency: USD, CommissionCurrToLocalExchangeRate: 1.2}

	delta2, err := AddTx(tx_, delta1.PostStatus)
	if err != nil {
		return
	}

	RenderTxTable([]*TxDelta{delta1, delta2})
}

func RenderTxTable(deltas []*TxDelta) {
	table := tw.NewWriter(os.Stdout)
	table.SetHeader([]string{"Security", "Date", "TX", "Amount", "Shares", "Amt/Share",
		"Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB", "New ACB/Share",
		"Memo",
	})
	table.SetBorder(false)
	table.SetRowLine(true)

	var capGainsTotal float64 = 0.0

	for _, d := range deltas {
		tx := d.Tx
		row := []string{d.Tx.Security, dateStr(tx.Date), tx.Action.String(),
			// Amount
			currWithFxStr(float64(tx.Shares)*tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			fmt.Sprintf("%d", tx.Shares),
			currWithFxStr(tx.AmountPerShare, tx.TxCurrency, tx.TxCurrToLocalExchangeRate),
			strOrDash(tx.Commission != 0.0,
				currWithFxStr(tx.Commission, tx.CommissionCurrency, tx.CommissionCurrToLocalExchangeRate)),
			strOrDash(tx.Action == SELL, plusMinusDollar(d.CapitalGain, false)),
			fmt.Sprintf("%d", d.PostStatus.ShareBalance),
			plusMinusDollar(d.AcbDelta(), true),
			"$" + currStr(d.PostStatus.TotalAcb),
			// Acb per share
			strOrDash(d.PostStatus.ShareBalance > 0.0,
				"$"+currStr(d.PostStatus.TotalAcb/float64(d.PostStatus.ShareBalance))),
			tx.Memo,
		}
		table.Append(row)

		capGainsTotal += d.CapitalGain
	}
	table.SetFooter([]string{"", "", "", "", "", "",
		"Total", plusMinusDollar(capGainsTotal, false), "", "", "", "", ""})

	table.Render()
}
