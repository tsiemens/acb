package portfolio

import (
	"fmt"

	"github.com/tsiemens/acb/util"
)

func AddTx(tx *Tx, preTxStatus *PortfolioSecurityStatus) (*TxDelta, error) {
	util.Assert(tx.Security == preTxStatus.Security)

	var totalLocalSharePrice float64 = float64(tx.Shares) * tx.PricePerShare * tx.TxCurrToLocalExchangeRate

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
		acbReduction := (tx.PricePerShare * float64(preTxStatus.ShareBalance) * tx.TxCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb - acbReduction
		if newAcbTotal < 0.0 {
			return nil, fmt.Errorf("Invalid RoC tx on %v: RoC (%f) exceeds the current ACB (%f)",
				tx.Date, acbReduction, preTxStatus.TotalAcb)
		}
	default:
		util.Assert(false, "Invalid action", tx.Action)
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
