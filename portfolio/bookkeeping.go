package portfolio

import (
	"fmt"

	"github.com/tsiemens/acb/date"
	"github.com/tsiemens/acb/util"
)

type LegacyOptions struct {
	NoSuperficialLosses        bool
	NoPartialSuperficialLosses bool
}

func NewLegacyOptions() LegacyOptions {
	return LegacyOptions{
		NoSuperficialLosses:        false,
		NoPartialSuperficialLosses: false,
	}
}

type _SuperficialLossInfo struct {
	IsSuperficial        bool
	FirstDateInPeriod    date.Date
	LastDateInPeriod     date.Date
	SharesAtEndOfPeriod  uint32
	TotalAquiredInPeriod uint32
}

func GetFirstDayInSuperficialLossPeriod(txDate date.Date) date.Date {
	return txDate.AddDays(-30)
}

func GetLastDayInSuperficialLossPeriod(txDate date.Date) date.Date {
	return txDate.AddDays(30)
}

// Checks if there is a Buy action within 30 days before or after the Sell
// at idx, AND if you hold shares after the 30 day period
// Also gathers relevant information for partial superficial loss calculation.
func getSuperficialLossInfo(idx int, txs []*Tx, shareBalanceAfterSell uint32) _SuperficialLossInfo {
	tx := txs[idx]
	util.Assertf(tx.Action == SELL,
		"getSuperficialLossInfo: Tx was not Sell, but %s", tx.Action)

	firstBadBuyDate := GetFirstDayInSuperficialLossPeriod(tx.Date)
	lastBadBuyDate := GetLastDayInSuperficialLossPeriod(tx.Date)

	sli := _SuperficialLossInfo{
		IsSuperficial:        false,
		FirstDateInPeriod:    firstBadBuyDate,
		LastDateInPeriod:     lastBadBuyDate,
		SharesAtEndOfPeriod:  shareBalanceAfterSell,
		TotalAquiredInPeriod: 0,
	}

	didBuyAfterInPeriod := false
	for i := idx + 1; i < len(txs); i++ {
		afterTx := txs[i]
		if afterTx.Date.After(lastBadBuyDate) {
			break
		}
		// Within the 30 day window after
		switch afterTx.Action {
		case BUY:
			didBuyAfterInPeriod = true
			sli.SharesAtEndOfPeriod += afterTx.Shares
			sli.TotalAquiredInPeriod += afterTx.Shares
		case SELL:
			sli.SharesAtEndOfPeriod -= afterTx.Shares
		default:
			// ignored
		}
	}

	if sli.SharesAtEndOfPeriod == 0 {
		// Not superficial
		return sli
	}

	didBuyBeforeInPeriod := false
	for i := idx - 1; i >= 0; i-- {
		beforeTx := txs[i]
		if beforeTx.Date.Before(firstBadBuyDate) {
			break
		}
		// Within the 30 day window before
		if beforeTx.Action == BUY {
			didBuyBeforeInPeriod = true
			sli.TotalAquiredInPeriod += beforeTx.Shares
		}
	}

	sli.IsSuperficial = didBuyBeforeInPeriod || didBuyAfterInPeriod
	return sli
}

// Calculation of partial superficial losses where
// Superficial loss = (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)
// This function returns the left hand side of this formula, on the condition that
// the loss is actually superficial.
//
// Reference: https://www.adjustedcostbase.ca/blog/applying-the-superficial-loss-rule-for-a-partial-disposition-of-shares/
func SuperficialLossPercent(idx int, txs []*Tx, shareBalanceAfterSell uint32) float64 {
	sli := getSuperficialLossInfo(idx, txs, shareBalanceAfterSell)

	if sli.IsSuperficial {
		tx := txs[idx]
		return float64(util.MinUint32(tx.Shares, sli.TotalAquiredInPeriod, sli.SharesAtEndOfPeriod)) / float64(tx.Shares)
	} else {
		return 0.0
	}
}

func AddTx(idx int, txs []*Tx, preTxStatus *PortfolioSecurityStatus, legacyOptions LegacyOptions) (*TxDelta, error) {
	applySuperficialLosses := !legacyOptions.NoSuperficialLosses
	noPartialSuperficialLosses := legacyOptions.NoPartialSuperficialLosses
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

		if capitalGains < 0.0 && applySuperficialLosses {
			superficialLossPercent := SuperficialLossPercent(idx, txs, newShareBalance)
			if superficialLossPercent != 0.0 {
				if noPartialSuperficialLosses {
					superficialLoss = capitalGains
					capitalGains = 0.0
				} else {
					superficialLoss = capitalGains * superficialLossPercent
					capitalGains = capitalGains - superficialLoss
				}
				newAcbTotal -= superficialLoss
			}
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

func TxsToDeltaList(txs []*Tx, initialStatus *PortfolioSecurityStatus, legacyOptions LegacyOptions) ([]*TxDelta, error) {
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
		delta, err := AddTx(i, txs, lastStatus, legacyOptions)
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
