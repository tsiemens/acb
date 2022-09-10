package portfolio

import (
	"fmt"
	"math"

	"github.com/tsiemens/acb/date"
	"github.com/tsiemens/acb/util"
)

type LegacyOptions struct {
	// None currently
}

func NewLegacyOptions() LegacyOptions {
	return LegacyOptions{}
}

func NonNilTxAffiliate(tx *Tx) *Affiliate {
	txAffiliate := tx.Affiliate
	if txAffiliate == nil {
		// This should only really happen in tests
		txAffiliate = GlobalAffiliateDedupTable.GetDefaultAffiliate()
	}
	return txAffiliate
}

type AffiliatePortfolioSecurityStatuses struct {
	// Affiliate Id -> last PortfolioSecurityStatus
	lastPostStatusForAffiliate      map[string]*PortfolioSecurityStatus
	security                        string
	latestAllAffiliatesShareBalance uint32
	latestAffiliate                 *Affiliate
}

func NewAffiliatePortfolioSecurityStatuses(
	security string, initialDefaultAffStatus *PortfolioSecurityStatus,
) *AffiliatePortfolioSecurityStatuses {

	s := &AffiliatePortfolioSecurityStatuses{
		lastPostStatusForAffiliate:      make(map[string]*PortfolioSecurityStatus),
		security:                        security,
		latestAllAffiliatesShareBalance: 0,
		latestAffiliate:                 GlobalAffiliateDedupTable.GetDefaultAffiliate(),
	}

	// Initial status only applies to the default affiliate
	if initialDefaultAffStatus != nil {
		util.Assert(initialDefaultAffStatus.ShareBalance ==
			initialDefaultAffStatus.AllAffiliatesShareBalance)
		s.SetLatestPostStatus(s.latestAffiliate.Id(), initialDefaultAffStatus)
	}
	return s
}

func (s *AffiliatePortfolioSecurityStatuses) makeDefaultPortfolioSecurityStatus(
	defaultAff bool, registered bool) *PortfolioSecurityStatus {
	var affiliateShareBalance uint32 = 0
	var affiliateTotalAcb float64 = util.Tern[float64](registered, math.NaN(), 0.0)
	return &PortfolioSecurityStatus{
		Security: s.security, ShareBalance: affiliateShareBalance,
		AllAffiliatesShareBalance: s.latestAllAffiliatesShareBalance,
		TotalAcb:                  affiliateTotalAcb,
	}
}

func (s *AffiliatePortfolioSecurityStatuses) GetLatestPostStatusForAffiliate(
	id string) (*PortfolioSecurityStatus, bool) {
	v, ok := s.lastPostStatusForAffiliate[id]
	return v, ok
}

func (s *AffiliatePortfolioSecurityStatuses) GetLatestPostStatus() *PortfolioSecurityStatus {
	v, ok := s.GetLatestPostStatusForAffiliate(s.latestAffiliate.Id())
	if !ok {
		return &PortfolioSecurityStatus{Security: s.security}
	}
	return v
}

func (s *AffiliatePortfolioSecurityStatuses) SetLatestPostStatus(
	id string, v *PortfolioSecurityStatus) {

	var lastShareBalance uint32 = 0
	if last, ok := s.lastPostStatusForAffiliate[id]; ok {
		lastShareBalance = last.ShareBalance
	}
	var expectedAllShareBal uint32 = v.ShareBalance + s.latestAllAffiliatesShareBalance - lastShareBalance

	af := GlobalAffiliateDedupTable.MustGet(id)
	util.Assertf(af.Registered() == math.IsNaN(v.TotalAcb),
		"In security %s, af %s, TotalAcb has bad NaN value (%f)",
		s.security, id, v.TotalAcb)

	util.Assertf(v.AllAffiliatesShareBalance == expectedAllShareBal,
		"In security %s, af %s, v.AllAffiliatesShareBalance (%d) != expectedAllShareBal (%d) "+
			"(v.ShareBalance (%d) + s.latestAllAffiliatesShareBalance (%d) - lastShareBalance (%d)",
		s.security, id, v.AllAffiliatesShareBalance, expectedAllShareBal,
		v.ShareBalance, s.latestAllAffiliatesShareBalance, lastShareBalance)

	s.lastPostStatusForAffiliate[id] = v
	s.latestAllAffiliatesShareBalance = v.AllAffiliatesShareBalance
	s.latestAffiliate = GlobalAffiliateDedupTable.MustGet(id)
}

func (s *AffiliatePortfolioSecurityStatuses) GetNextPreStatus(
	id string) *PortfolioSecurityStatus {

	lastStatus, ok := s.GetLatestPostStatusForAffiliate(id)
	if !ok {
		af := GlobalAffiliateDedupTable.MustGet(id)
		lastStatus = s.makeDefaultPortfolioSecurityStatus(af.Default(), af.Registered())
	}
	nextPreStatus := lastStatus
	if nextPreStatus.AllAffiliatesShareBalance != s.latestAllAffiliatesShareBalance {
		nextPreStatus = &PortfolioSecurityStatus{}
		*nextPreStatus = *lastStatus
		nextPreStatus.AllAffiliatesShareBalance = s.latestAllAffiliatesShareBalance
	}
	return nextPreStatus
}

type _SuperficialLossInfo struct {
	IsSuperficial        bool
	FirstDateInPeriod    date.Date
	LastDateInPeriod     date.Date
	SharesAtEndOfPeriod  uint32
	TotalAquiredInPeriod uint32
	BuyingAffiliates     *util.Set[string]
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
func getSuperficialLossInfo(
	idx int, txs []*Tx, allAffiliatesShareBalanceAfterSell uint32) _SuperficialLossInfo {
	tx := txs[idx]
	util.Assertf(tx.Action == SELL,
		"getSuperficialLossInfo: Tx was not Sell, but %s", tx.Action)

	firstBadBuyDate := GetFirstDayInSuperficialLossPeriod(tx.SettlementDate)
	lastBadBuyDate := GetLastDayInSuperficialLossPeriod(tx.SettlementDate)

	sli := _SuperficialLossInfo{
		IsSuperficial:        false,
		FirstDateInPeriod:    firstBadBuyDate,
		LastDateInPeriod:     lastBadBuyDate,
		SharesAtEndOfPeriod:  allAffiliatesShareBalanceAfterSell,
		TotalAquiredInPeriod: 0,
	}

	buyingAffiliates := util.NewSet[string]()
	// TODO fix SFL logic for multiple affiliates.
	// Some points:
	// the total share balance across all affiliates is insufficient, since
	// if you had 3 affiliates, it's possible to retain shares, but in an affiliate
	// which did not do any of the buys within the period. This should probably
	// require a manual entry, since I don't know what to do in this case. Is the
	// loss denied or not? Is the total number of shares only for the affiliates
	// doing the sell and with the buys?
	// I think the total shares should only be in the affiliates which did the
	// sell.
	// Do we use the shares left in the affiliate with the buy only?
	// hypothetical:
	//  A                 B
	//  BUY 5				BUY 0
	//	 ...              ...
	//  SELL 4 (SFL)		BUY 5
	//							SELL 3
	// (reminaing: 1)		(remaining: 2)
	// use 2 or 3 as remaining shares, since it is the min val for proportional SFL.
	//
	// However, the safer thing to do might be to use the max shares, but require
	// manual entry if the number of shares remaining in the sell affiliate is less
	// than the number of rejected loss shares. <<<<<< Warn of this and possibly suggest an accountant.

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
			// TODO need to fix tests so this is never nil
			txAffil := tx.Affiliate
			if txAffil == nil {
				txAffil = GlobalAffiliateDedupTable.GetDefaultAffiliate()
			}
			buyingAffiliates.Add(txAffil.Id())
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

	sli.BuyingAffiliates = buyingAffiliates
	sli.IsSuperficial = didBuyBeforeInPeriod || didBuyAfterInPeriod
	return sli
}

// Calculation of partial superficial losses where
// Superficial loss = (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)
// This function returns the left hand side of this formula, on the condition that
// the loss is actually superficial.
//
// Reference: https://www.adjustedcostbase.ca/blog/applying-the-superficial-loss-rule-for-a-partial-disposition-of-shares/
func getSuperficialLossRatio(
	idx int, txs []*Tx, allAffiliatesShareBalanceAfterSell uint32) util.Uint32Ratio {
	sli := getSuperficialLossInfo(idx, txs, allAffiliatesShareBalanceAfterSell)

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
) (*TxDelta, *Tx, error) {

	tx := txs[idx]
	util.Assertf(tx.Security == preTxStatus.Security,
		"AddTx: securities do not match (%s and %s)\n", tx.Security, preTxStatus.Security)

	var totalLocalSharePrice float64 = float64(tx.Shares) * tx.AmountPerShare * tx.TxCurrToLocalExchangeRate

	newShareBalance := preTxStatus.ShareBalance
	newAllAffiliatesShareBalance := preTxStatus.AllAffiliatesShareBalance
	registered := tx.Affiliate != nil && tx.Affiliate.Registered()
	var newAcbTotal float64 = preTxStatus.TotalAcb
	var capitalGains float64 = util.Tern[float64](registered, math.NaN(), 0.0)
	var superficialLoss float64 = util.Tern[float64](registered, math.NaN(), 0.0)
	superficialLossRatio := util.Uint32Ratio{}
	var newTx *Tx = nil

	// Sanity checks
	sanityCheckError := func(fmtStr string, v ...interface{}) error {
		return fmt.Errorf(
			"In transaction on %v of %d shares of %s, "+fmtStr,
			append([]interface{}{tx.TradeDate, tx.Shares, tx.Security}, v...)...)
	}
	if preTxStatus.AllAffiliatesShareBalance < preTxStatus.ShareBalance {
		return nil, nil, sanityCheckError("the share balance across all affiliates "+
			"(%d) is lower than the share balance for the affiliate of the sale (%d)",
			preTxStatus.AllAffiliatesShareBalance, preTxStatus.ShareBalance)
	} else if registered && !math.IsNaN(preTxStatus.TotalAcb) {
		return nil, nil, sanityCheckError("found an ACB on a registered affiliate")
	} else if !registered && math.IsNaN(preTxStatus.TotalAcb) {
		return nil, nil, sanityCheckError("found an invalid ACB (NaN)")
	}

	switch tx.Action {
	case BUY:
		newShareBalance = preTxStatus.ShareBalance + tx.Shares
		newAllAffiliatesShareBalance = preTxStatus.AllAffiliatesShareBalance + tx.Shares
		totalPrice := totalLocalSharePrice + (tx.Commission * tx.CommissionCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb + (totalPrice)
	case SELL:
		if tx.Shares > preTxStatus.ShareBalance {
			return nil, nil, fmt.Errorf(
				"Sell order on %v of %d shares of %s is more than the current holdings (%d)",
				tx.TradeDate, tx.Shares, tx.Security, preTxStatus.ShareBalance)
		}
		newShareBalance = preTxStatus.ShareBalance - tx.Shares
		newAllAffiliatesShareBalance = preTxStatus.AllAffiliatesShareBalance - tx.Shares
		// Note commission plays no effect on sell order ACB
		newAcbTotal = preTxStatus.TotalAcb - (preTxStatus.PerShareAcb() * float64(tx.Shares))
		totalPayout := totalLocalSharePrice - (tx.Commission * tx.CommissionCurrToLocalExchangeRate)
		capitalGains = totalPayout - (preTxStatus.PerShareAcb() * float64(tx.Shares))

		if !registered && capitalGains < 0.0 {
			superficialLossRatio = getSuperficialLossRatio(
				idx, txs, newAllAffiliatesShareBalance)
			calculatedSuperficialLoss := 0.0
			if superficialLossRatio.Valid() {
				calculatedSuperficialLoss = capitalGains * superficialLossRatio.ToFloat64()
			}

			if tx.SpecifiedSuperficialLoss.Present() {
				superficialLoss = tx.SpecifiedSuperficialLoss.MustGet().SuperficialLoss
				capitalGains = capitalGains - superficialLoss

				if !tx.SpecifiedSuperficialLoss.MustGet().Force {
					sflDiff := math.Abs(calculatedSuperficialLoss - superficialLoss)
					const maxDiff float64 = 0.001
					if sflDiff > maxDiff {
						return nil, nil, fmt.Errorf(
							"Sell order on %v of %s: superficial loss was specified, but "+
								"the difference between the specified value (%f) and the "+
								"computed value (%f) is greater than the max allowed "+
								"discrepancy (%f).\nTo force this SFL value, append an '!' "+
								"to the value",
							tx.TradeDate, tx.Security, superficialLoss,
							calculatedSuperficialLoss, maxDiff)
					}
				}

				// ACB adjustment TX must be specified manually in this case.
			} else if superficialLossRatio.Valid() {
				superficialLoss = calculatedSuperficialLoss
				capitalGains = capitalGains - calculatedSuperficialLoss

				// This new Tx will adjust (increase) the ACB for this superficial loss.
				newTx = &Tx{
					Security:                  tx.Security,
					TradeDate:                 tx.TradeDate,
					SettlementDate:            tx.SettlementDate,
					Action:                    SFLA,
					Shares:                    superficialLossRatio.Numerator,
					AmountPerShare:            -1.0 * superficialLoss / float64(superficialLossRatio.Numerator),
					TxCurrency:                CAD,
					TxCurrToLocalExchangeRate: 1.0,
					Memo:                      "automatic SfL ACB adjustment",
					// TODO this should be the affiliate for which all buys occurred.
					// If buys are spread between affiliates, we should not be automatically
					// adding this adjustment, and require the user to specify the SFL on the sell.
					Affiliate: tx.Affiliate,
				}
			}
		} else if tx.SpecifiedSuperficialLoss.Present() {
			return nil, nil, fmt.Errorf(
				"Sell order on %v of %s: superficial loss was specified, but there is no capital loss",
				tx.TradeDate, tx.Security)
		}
	case ROC:
		if registered {
			return nil, nil, fmt.Errorf(
				"Invalid RoC tx on %v: Registered affiliates do not have an ACB to adjust",
				tx.TradeDate)
		}
		if tx.Shares != 0 {
			return nil, nil, fmt.Errorf("Invalid RoC tx on %v: # of shares is non-zero (%d)",
				tx.TradeDate, tx.Shares)
		}
		acbReduction := tx.AmountPerShare * float64(preTxStatus.ShareBalance) *
			tx.TxCurrToLocalExchangeRate
		newAcbTotal = preTxStatus.TotalAcb - acbReduction
		if newAcbTotal < 0.0 {
			return nil, nil, fmt.Errorf("Invalid RoC tx on %v: RoC (%f) exceeds the current ACB (%f)",
				tx.TradeDate, acbReduction, preTxStatus.TotalAcb)
		}
	case SFLA:
		if registered {
			return nil, nil, fmt.Errorf(
				"Invalid SfLA tx on %v: Registered affiliates do not have an ACB to adjust",
				tx.TradeDate)
		}
		acbAdjustment := tx.AmountPerShare * float64(tx.Shares) *
			tx.TxCurrToLocalExchangeRate
		newAcbTotal = preTxStatus.TotalAcb + acbAdjustment
		if !(tx.TxCurrency == CAD || tx.TxCurrency == DEFAULT_CURRENCY) ||
			tx.TxCurrToLocalExchangeRate != 1.0 {
			return nil, nil, fmt.Errorf(
				"Invalid SfLA tx on %v: Currency is not CAD/default, and/or exchange rate is not 1",
				tx.TradeDate)
		}
	default:
		util.Assertf(false, "Invalid action: %v\n", tx.Action)
	}

	newStatus := &PortfolioSecurityStatus{
		Security:                  preTxStatus.Security,
		ShareBalance:              newShareBalance,
		AllAffiliatesShareBalance: newAllAffiliatesShareBalance,
		TotalAcb:                  newAcbTotal,
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

	var modifiedTxs []*Tx
	activeTxs := txs
	deltas := make([]*TxDelta, 0, len(txs))

	if len(txs) == 0 {
		return deltas, nil
	}

	// TODO give this to AddTx
	lastStatusForAffiliates := NewAffiliatePortfolioSecurityStatuses(
		txs[0].Security, initialStatus)

	for i := 0; i < len(activeTxs); i++ {
		txAffiliate := NonNilTxAffiliate(activeTxs[i])
		preTxStatus := lastStatusForAffiliates.GetNextPreStatus(txAffiliate.Id())
		delta, newTx, err := AddTx(i, activeTxs, preTxStatus)
		if err != nil {
			// Return what we've managed so far, for debugging
			return deltas, err
		}
		lastStatusForAffiliates.SetLatestPostStatus(txAffiliate.Id(), delta.PostStatus)
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
