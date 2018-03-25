package portfolio

import (
	"fmt"

	"github.com/tsiemens/acb/util"
)

func AddTx(tx *Tx, preTxStatus *PortfolioSecurityStatus) (*TxDelta, error) {
	util.Assertf(tx.Security == preTxStatus.Security,
		"AddTx: securities do not match (%s and %s)\n", tx.Security, preTxStatus.Security)

	var totalLocalSharePrice float64 = float64(tx.Shares) * tx.AmountPerShare * tx.TxCurrToLocalExchangeRate

	newShareBalance := preTxStatus.ShareBalance
	var newAcbTotal float64 = preTxStatus.TotalAcb
	var capitalGains float64 = 0.0

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
		Tx:          tx,
		PreStatus:   preTxStatus,
		PostStatus:  newStatus,
		CapitalGain: capitalGains,
	}
	return delta, nil
}

func TxsToDeltaList(txs []*Tx, initialStatus *PortfolioSecurityStatus) ([]*TxDelta, error) {
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
	for _, tx := range txs {
		delta, err := AddTx(tx, lastStatus)
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
