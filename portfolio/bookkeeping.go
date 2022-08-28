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

	firstBadBuyDate := GetFirstDayInSuperficialLossPeriod(tx.SettlementDate)
	lastBadBuyDate := GetLastDayInSuperficialLossPeriod(tx.SettlementDate)

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
		if afterTx.SettlementDate.After(lastBadBuyDate) {
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
		if beforeTx.SettlementDate.Before(firstBadBuyDate) {
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
func getSuperficialLossRatio(idx int, txs []*Tx, shareBalanceAfterSell uint32) util.Uint32Ratio {
	sli := getSuperficialLossInfo(idx, txs, shareBalanceAfterSell)

	if sli.IsSuperficial {
		tx := txs[idx]
		return util.Uint32Ratio{
			util.MinUint32(tx.Shares, sli.TotalAquiredInPeriod, sli.SharesAtEndOfPeriod),
			tx.Shares,
		}
	} else {
		return util.Uint32Ratio{}
	}
}

// Returns a TxDelta for the Tx at txs[idx].
// Optionally, returns a new Tx if a SFLA Tx was generated to accompany
// this Tx. It is expected that that Tx be inserted into txs and evaluated next.
func AddTx(
	idx int,
	txs []*Tx,
	preTxStatus *PortfolioSecurityStatus,
	legacyOptions LegacyOptions,
) (*TxDelta, *Tx, error) {

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
	superficialLossRatio := util.Uint32Ratio{}
	var newTx *Tx = nil

	switch tx.Action {
	case BUY:
		newShareBalance = preTxStatus.ShareBalance + tx.Shares
		totalPrice := totalLocalSharePrice + (tx.Commission * tx.CommissionCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb + (totalPrice)
	case SELL:
		if tx.Shares > preTxStatus.ShareBalance {
			return nil, nil, fmt.Errorf(
				"Sell order on %v of %d shares of %s is more than the current holdings (%d)",
				tx.SettlementDate, tx.Shares, tx.Security, preTxStatus.ShareBalance)
		}
		newShareBalance = preTxStatus.ShareBalance - tx.Shares
		// Note commission plays no effect on sell order ACB
		newAcbTotal = preTxStatus.TotalAcb - (preTxStatus.PerShareAcb() * float64(tx.Shares))
		totalPayout := totalLocalSharePrice - (tx.Commission * tx.CommissionCurrToLocalExchangeRate)
		capitalGains = totalPayout - (preTxStatus.PerShareAcb() * float64(tx.Shares))

		// TODO Check for SpecifiedSuperficialLoss

		if capitalGains < 0.0 && applySuperficialLosses {
			superficialLossRatio = getSuperficialLossRatio(idx, txs, newShareBalance)
			if superficialLossRatio.Valid() {
				if noPartialSuperficialLosses {
					superficialLossRatio.Numerator = superficialLossRatio.Denominator
					superficialLoss = capitalGains
					capitalGains = 0.0
				} else {
					superficialLoss = capitalGains * superficialLossRatio.ToFloat64()
					capitalGains = capitalGains - superficialLoss
				}

				// This new Tx will adjust (increase) the ACB for this superficial loss.
				newTx = &Tx{
					Security:                  tx.Security,
					TradeDate:                 tx.TradeDate,
					SettlementDate:            tx.SettlementDate,
					Action:                    SFLA,
					Shares:                    superficialLossRatio.Numerator,
					AmountPerShare:            -1.0 * superficialLoss / float64(superficialLossRatio.Numerator),
					TxCurrency:                tx.TxCurrency,
					TxCurrToLocalExchangeRate: tx.TxCurrToLocalExchangeRate,
					Memo:                      "automatic SfL ACB adjustment",
				}
			}
		}
	case ROC:
		if tx.Shares != 0 {
			return nil, nil, fmt.Errorf("Invalid RoC tx on %v: # of shares is non-zero (%d)",
				tx.SettlementDate, tx.Shares)
		}
		acbReduction := tx.AmountPerShare * float64(preTxStatus.ShareBalance) *
			tx.TxCurrToLocalExchangeRate
		newAcbTotal = preTxStatus.TotalAcb - acbReduction
		if newAcbTotal < 0.0 {
			return nil, nil, fmt.Errorf("Invalid RoC tx on %v: RoC (%f) exceeds the current ACB (%f)",
				tx.SettlementDate, acbReduction, preTxStatus.TotalAcb)
		}
	case SFLA:
		acbAdjustment := tx.AmountPerShare * float64(tx.Shares) *
			tx.TxCurrToLocalExchangeRate
		newAcbTotal = preTxStatus.TotalAcb + acbAdjustment
	default:
		util.Assertf(false, "Invalid action: %v\n", tx.Action)
	}

	newStatus := &PortfolioSecurityStatus{
		Security:     preTxStatus.Security,
		ShareBalance: newShareBalance,
		TotalAcb:     newAcbTotal,
	}
	delta := &TxDelta{
		Tx:                   tx,
		PreStatus:            preTxStatus,
		PostStatus:           newStatus,
		CapitalGain:          capitalGains,
		SuperficialLoss:      superficialLoss,
		SuperficialLossRatio: superficialLossRatio,
	}
	return delta, newTx, nil
}

// Insert tx at index i and return the resulting slice
func insertTx(slice []*Tx, tx *Tx, i int) []*Tx {
	newSlice := make([]*Tx, 0, len(slice)+1)
	newSlice = append(newSlice, slice[:i]...)
	newSlice = append(newSlice, tx)
	newSlice = append(newSlice, slice[i:]...)
	return newSlice
}

func TxsToDeltaList(
	txs []*Tx,
	initialStatus *PortfolioSecurityStatus,
	legacyOptions LegacyOptions,
) ([]*TxDelta, error) {

	if initialStatus == nil {
		if len(txs) == 0 {
			return []*TxDelta{}, nil
		}
		initialStatus = &PortfolioSecurityStatus{
			Security: txs[0].Security, ShareBalance: 0, TotalAcb: 0.0,
		}
	}

	var modifiedTxs []*Tx
	activeTxs := txs
	deltas := make([]*TxDelta, 0, len(txs))
	lastStatus := initialStatus

	for i := 0; i < len(activeTxs); i++ {
		delta, newTx, err := AddTx(i, activeTxs, lastStatus, legacyOptions)
		if err != nil {
			// Return what we've managed so far, for debugging
			return deltas, err
		}
		lastStatus = delta.PostStatus
		deltas = append(deltas, delta)
		if newTx != nil {
			// Add new Tx into modifiedTxs
			if modifiedTxs == nil {
				// Copy Txs, as we now need to modify
				modifiedTxs = make([]*Tx, 0, len(txs))
				modifiedTxs = append(modifiedTxs, txs...)
				activeTxs = modifiedTxs
			}
			// Insert into modifiedTxs after the current Tx
			modifiedTxs = insertTx(modifiedTxs, newTx, i+1)
			activeTxs = modifiedTxs
		}
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
