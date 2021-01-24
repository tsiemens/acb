package portfolio

import (
	"fmt"
	"time"

	"github.com/tsiemens/acb/util"
)

func mustParseDuration(str string) time.Duration {
	dur, err := time.ParseDuration(str)
	util.Assert(err == nil, err)
	return dur
}

var ONE_DAY_DUR = mustParseDuration("24h")

// Checks if there is a Buy action within 30 days before or after the Sell
// at idx, AND if you hold shares after the 30 day period
//
// NOTE: Currently this only supports FULL superficial loss application. It cannot
// apply partial superficial losses yet. For more info on partial superficial losses,
// see https://www.adjustedcostbase.ca/blog/applying-the-superficial-loss-rule-for-a-partial-disposition-of-shares/
func IsSellSuperficial(idx int, txs []*Tx, shareBalanceAfterSell uint32) bool {
	tx := txs[idx]
	util.Assertf(tx.Action == SELL,
		"IsSellSuperficial: Tx was not Sell, but %s", tx.Action)

	firstBadBuyDate := tx.Date.Add(-30 * ONE_DAY_DUR)
	lastBadBuyDate := tx.Date.Add(30 * ONE_DAY_DUR)

	shareBalanceAfterPeriod := shareBalanceAfterSell
	didBuyAfter := false
	for i := idx + 1; i < len(txs); i++ {
		afterTx := txs[i]
		if afterTx.Date.After(lastBadBuyDate) {
			break
		}
		// Within the 30 day window after
		switch afterTx.Action {
		case BUY:
			didBuyAfter = true
			shareBalanceAfterPeriod += afterTx.Shares
		case SELL:
			shareBalanceAfterPeriod -= afterTx.Shares
		default:
			// ignored
		}
	}

	if shareBalanceAfterPeriod == 0 {
		return false
	} else if didBuyAfter {
		return true
	}

	for i := idx - 1; i >= 0; i-- {
		if txs[i].Date.Before(firstBadBuyDate) {
			break
		}
		// Within the 30 day window before
		if txs[i].Action == BUY {
			return true
		}
	}

	return false
}

func AddTx(idx int, txs []*Tx, preTxStatus *PortfolioSecurityStatus, applySuperficialLosses bool) (*TxDelta, error) {
	tx := txs[idx]
	util.Assertf(tx.Security == preTxStatus.Security,
		"AddTx: securities do not match (%s and %s)\n", tx.Security, preTxStatus.Security)

	var totalLocalSharePrice float64 = float64(tx.Shares) * tx.AmountPerShare * tx.TxCurrToLocalExchangeRate

	newShareBalance := preTxStatus.ShareBalance
	var newAcbTotal float64 = preTxStatus.TotalAcb
	var capitalGains float64 = 0.0
	var superficialLoss float64 = 0.0

	switch tx.Action {
	case BUY:
		newShareBalance = preTxStatus.ShareBalance + tx.Shares
		totalPrice := totalLocalSharePrice + (tx.Commission * tx.CommissionCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb + (totalPrice)
	case SELL:
		if tx.Shares > preTxStatus.ShareBalance {
			return nil, fmt.Errorf("Sell order on %v of %d shares of %s is more than the current holdings (%d)",
				tx.Date, tx.Shares, tx.Security, preTxStatus.ShareBalance)
		}
		newShareBalance = preTxStatus.ShareBalance - tx.Shares
		// Note commission plays no effect on sell order ACB
		newAcbTotal = preTxStatus.TotalAcb - (preTxStatus.PerShareAcb() * float64(tx.Shares))
		totalPayout := totalLocalSharePrice - (tx.Commission * tx.CommissionCurrToLocalExchangeRate)
		capitalGains = totalPayout - (preTxStatus.PerShareAcb() * float64(tx.Shares))

		if capitalGains < 0.0 &&
			applySuperficialLosses && IsSellSuperficial(idx, txs, newShareBalance) {

			superficialLoss = capitalGains
			capitalGains = 0.0
			newAcbTotal -= superficialLoss
		}
	case ROC:
		if tx.Shares != 0 {
			return nil, fmt.Errorf("Invalid RoC tx on %v: # of shares is non-zero (%d)",
				tx.Date, tx.Shares)
		}
		acbReduction := (tx.AmountPerShare * float64(preTxStatus.ShareBalance) * tx.TxCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb - acbReduction
		if newAcbTotal < 0.0 {
			return nil, fmt.Errorf("Invalid RoC tx on %v: RoC (%f) exceeds the current ACB (%f)",
				tx.Date, acbReduction, preTxStatus.TotalAcb)
		}
	default:
		util.Assertf(false, "Invalid action: %v\n", tx.Action)
	}

	newStatus := &PortfolioSecurityStatus{
		Security:     preTxStatus.Security,
		ShareBalance: newShareBalance,
		TotalAcb:     newAcbTotal,
	}
	delta := &TxDelta{
		Tx:              tx,
		PreStatus:       preTxStatus,
		PostStatus:      newStatus,
		CapitalGain:     capitalGains,
		SuperficialLoss: superficialLoss,
	}
	return delta, nil
}

func TxsToDeltaList(txs []*Tx, initialStatus *PortfolioSecurityStatus, applySuperficialLosses bool) ([]*TxDelta, error) {
	if initialStatus == nil {
		if len(txs) == 0 {
			return []*TxDelta{}, nil
		}
		initialStatus = &PortfolioSecurityStatus{
			Security: txs[0].Security, ShareBalance: 0, TotalAcb: 0.0,
		}
	}

	deltas := make([]*TxDelta, 0, len(txs))
	lastStatus := initialStatus
	for i, _ := range txs {
		delta, err := AddTx(i, txs, lastStatus, applySuperficialLosses)
		if err != nil {
			// Return what we've managed so far, for debugging
			return deltas, err
		}
		lastStatus = delta.PostStatus
		deltas = append(deltas, delta)
	}
	return deltas, nil
}

func SplitTxsBySecurity(txs []*Tx) map[string][]*Tx {
	txsBySec := make(map[string][]*Tx)
	for _, tx := range txs {
		secTxs, ok := txsBySec[tx.Security]
		if !ok {
			secTxs = make([]*Tx, 0, 8)
		}
		secTxs = append(secTxs, tx)
		txsBySec[tx.Security] = secTxs
	}
	return txsBySec
}
