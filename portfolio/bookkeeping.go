package portfolio

import (
	"fmt"
	"sort"

	"github.com/shopspring/decimal"

	"github.com/tsiemens/acb/date"
	decimal_opt "github.com/tsiemens/acb/decimal_value"
	"github.com/tsiemens/acb/log"
	"github.com/tsiemens/acb/util"
)

const traceTag = "bookkeeping"

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
	latestAllAffiliatesShareBalance decimal.Decimal
	latestAffiliate                 *Affiliate
}

func NewAffiliatePortfolioSecurityStatuses(
	security string, initialDefaultAffStatus *PortfolioSecurityStatus,
) *AffiliatePortfolioSecurityStatuses {

	s := &AffiliatePortfolioSecurityStatuses{
		lastPostStatusForAffiliate:      make(map[string]*PortfolioSecurityStatus),
		security:                        security,
		latestAllAffiliatesShareBalance: decimal.Zero,
		latestAffiliate:                 GlobalAffiliateDedupTable.GetDefaultAffiliate(),
	}

	// Initial status only applies to the default affiliate
	if initialDefaultAffStatus != nil {
		util.Assert(
			initialDefaultAffStatus.ShareBalance.Equal(initialDefaultAffStatus.AllAffiliatesShareBalance),
		)
		s.SetLatestPostStatus(s.latestAffiliate.Id(), initialDefaultAffStatus)
	}
	return s
}

func (s *AffiliatePortfolioSecurityStatuses) makeDefaultPortfolioSecurityStatus(
	defaultAff bool, registered bool) *PortfolioSecurityStatus {
	var affiliateShareBalance decimal.Decimal
	var affiliateTotalAcb decimal_opt.DecimalOpt = util.Tern[decimal_opt.DecimalOpt](
		registered, decimal_opt.Null, decimal_opt.NewFromInt(0))
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

	var lastShareBalance decimal.Decimal
	if last, ok := s.lastPostStatusForAffiliate[id]; ok {
		lastShareBalance = last.ShareBalance
	}
	var expectedAllShareBal decimal.Decimal = v.ShareBalance.Add(s.latestAllAffiliatesShareBalance).Sub(lastShareBalance)

	af := GlobalAffiliateDedupTable.MustGet(id)
	util.Assertf(af.Registered() == v.TotalAcb.IsNull,
		"In security %s, af %s, TotalAcb has bad null optional value (%s)",
		s.security, id, v.TotalAcb.String())

	util.Assertf(v.AllAffiliatesShareBalance.Equal(expectedAllShareBal),
		"In security %s, af %s, v.AllAffiliatesShareBalance (%s) != expectedAllShareBal (%s) "+
			"(v.ShareBalance (%s) + s.latestAllAffiliatesShareBalance (%s) - lastShareBalance (%s)",
		s.security, id, v.AllAffiliatesShareBalance.String(), expectedAllShareBal.String(),
		v.ShareBalance.String(), s.latestAllAffiliatesShareBalance.String(), lastShareBalance.String())

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
	if !nextPreStatus.AllAffiliatesShareBalance.Equal(s.latestAllAffiliatesShareBalance) {
		nextPreStatus = &PortfolioSecurityStatus{}
		*nextPreStatus = *lastStatus
		nextPreStatus.AllAffiliatesShareBalance = s.latestAllAffiliatesShareBalance
	}
	return nextPreStatus
}

type _SuperficialLossInfo struct {
	IsSuperficial              bool
	FirstDateInPeriod          date.Date
	LastDateInPeriod           date.Date
	AllAffSharesAtEndOfPeriod  decimal.Decimal
	TotalAquiredInPeriod       decimal.Decimal
	BuyingAffiliates           *util.Set[string]
	ActiveAffiliateSharesAtEOP *util.DefaultMap[string, decimal.Decimal]
}

func (i *_SuperficialLossInfo) BuyingAffiliateSharesAtEOPTotal() decimal.Decimal {
	total := decimal.Zero
	i.BuyingAffiliates.ForEach(func(afId string) bool {
		total = total.Add(i.ActiveAffiliateSharesAtEOP.Get(afId))
		return true
	})
	return total
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
	idx int, txs []*Tx, ptfStatuses *AffiliatePortfolioSecurityStatuses) _SuperficialLossInfo {
	tx := txs[idx]
	util.Assertf(tx.Action == SELL,
		"getSuperficialLossInfo: Tx was not Sell, but %s", tx.Action)

	firstBadBuyDate := GetFirstDayInSuperficialLossPeriod(tx.SettlementDate)
	lastBadBuyDate := GetLastDayInSuperficialLossPeriod(tx.SettlementDate)

	latestPostStatus := ptfStatuses.GetLatestPostStatus()
	// The enclosing AddTx logic should have already caught this.
	util.Assertf(latestPostStatus.AllAffiliatesShareBalance.GreaterThanOrEqual(tx.Shares),
		"getSuperficialLossInfo: latest AllAffiliatesShareBalance (%d) is less than sold shares (%d)",
		latestPostStatus.AllAffiliatesShareBalance.String(), tx.Shares.String())
	allAffiliatesShareBalanceAfterSell :=
		ptfStatuses.GetLatestPostStatus().AllAffiliatesShareBalance.Sub(tx.Shares)

	activeAffiliateSharesAtEOP := util.NewDefaultMap[string, decimal.Decimal](
		// Default to post-sale share balance for the affiliate.
		func(afId string) decimal.Decimal {
			sellTxAffil := NonNilTxAffiliate(tx)
			if st, ok := ptfStatuses.GetLatestPostStatusForAffiliate(afId); ok {
				if afId == sellTxAffil.Id() {
					// The latest post status for the selling affiliate is not yet
					// saved, so recompute the post-sale share balance.
					// AddTx would have encountered an oversell if this was to assert.
					util.Assertf(st.ShareBalance.GreaterThanOrEqual(tx.Shares),
						"getSuperficialLossInfo: latest ShareBalance (%d) for affiliate (%s) "+
							"is less than sold shares (%d)",
						st.ShareBalance, sellTxAffil.Name(), tx.Shares.String())
					return st.ShareBalance.Sub(tx.Shares)
				}
				return st.ShareBalance
			}
			// No TXs for this affiliate before or at the current sell.
			// AddTx would have encountered an oversell if this was to assert.
			util.Assert(afId != sellTxAffil.Id(),
				"getSuperficialLossInfo: no existing portfolio status for affiliate %s",
				sellTxAffil.Name())
			return decimal.Zero
		})

	sli := _SuperficialLossInfo{
		IsSuperficial:              false,
		FirstDateInPeriod:          firstBadBuyDate,
		LastDateInPeriod:           lastBadBuyDate,
		AllAffSharesAtEndOfPeriod:  allAffiliatesShareBalanceAfterSell,
		TotalAquiredInPeriod:       decimal.Zero,
		BuyingAffiliates:           util.NewSet[string](),
		ActiveAffiliateSharesAtEOP: activeAffiliateSharesAtEOP,
	}

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
		afterTxAffil := NonNilTxAffiliate(afterTx)

		// Within the 30 day window after
		switch afterTx.Action {
		case BUY:
			didBuyAfterInPeriod = true
			sli.AllAffSharesAtEndOfPeriod = sli.AllAffSharesAtEndOfPeriod.Add(afterTx.Shares)
			activeAffiliateSharesAtEOP.Set(afterTxAffil.Id(),
				activeAffiliateSharesAtEOP.Get(afterTxAffil.Id()).Add(afterTx.Shares))
			sli.TotalAquiredInPeriod = sli.TotalAquiredInPeriod.Add(afterTx.Shares)
			sli.BuyingAffiliates.Add(afterTxAffil.Id())
		case SELL:
			sli.AllAffSharesAtEndOfPeriod = sli.AllAffSharesAtEndOfPeriod.Sub(afterTx.Shares)
			activeAffiliateSharesAtEOP.Set(afterTxAffil.Id(),
				activeAffiliateSharesAtEOP.Get(afterTxAffil.Id()).Sub(afterTx.Shares))
		default:
			// ignored
		}
	}

	if sli.AllAffSharesAtEndOfPeriod.IsZero() {
		// Not superficial
		return sli
	}

	didBuyBeforeInPeriod := false
	for i := idx - 1; i >= 0; i-- {
		beforeTx := txs[i]
		if beforeTx.SettlementDate.Before(firstBadBuyDate) {
			break
		}
		beforeTxAffil := NonNilTxAffiliate(beforeTx)
		// Within the 30 day window before
		if beforeTx.Action == BUY {
			didBuyBeforeInPeriod = true
			sli.TotalAquiredInPeriod = sli.TotalAquiredInPeriod.Add(beforeTx.Shares)
			sli.BuyingAffiliates.Add(beforeTxAffil.Id())
		}
	}

	sli.IsSuperficial = didBuyBeforeInPeriod || didBuyAfterInPeriod
	return sli
}

type _SflRatioResultResult struct {
	SflRatio                 util.DecimalRatio
	AcbAdjustAffiliateRatios map[string]util.DecimalRatio
	// ** Notes/warnings to emit later. **
	// Set when the sum of remaining involved affiliate shares is fewer than
	// the SFL shares, which means that the selling affiliate probably had some
	// shares they didn't sell. This can happen because we use interpretation/algo I.1
	// rather than I.2 (see the sfl wiki page) to determine the loss ratio.
	FewerRemainingSharesThanSflShares bool
}

// Calculation of partial superficial losses where
// Superficial loss = (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)
// This function returns the left hand side of this formula, on the condition that
// the loss is actually superficial.
//
// Returns:
// - the superficial loss ratio (if calculable)
// - the affiliate to apply an automatic adjustment to (if possible)
// - an soft error (warning), which only applies when auto-generating the SfLA
//
// Uses interpretation I.1 from the link below for splitting loss adjustments.
//
// More detailed discussions about adjustment allocation can be found at
// https://github.com/tsiemens/acb/wiki/Superficial-Losses
//
// Reference: https://www.adjustedcostbase.ca/blog/applying-the-superficial-loss-rule-for-a-partial-disposition-of-shares/
func getSuperficialLossRatio(
	idx int, txs []*Tx, ptfStatuses *AffiliatePortfolioSecurityStatuses) *_SflRatioResultResult {
	sli := getSuperficialLossInfo(idx, txs, ptfStatuses)
	if sli.IsSuperficial {
		tx := txs[idx]

		ratio := util.DecimalRatio{
			decimal.Min(tx.Shares, sli.TotalAquiredInPeriod, sli.AllAffSharesAtEndOfPeriod),
			tx.Shares,
		}

		util.Assertf(sli.BuyingAffiliates.Len() != 0,
			"getSuperficialLossRatio: loss was superficial, but no buying affiliates")

		// Affiliate to percentage of the SFL adjustment is attributed to it.
		affiliateAdjustmentPortions := make(map[string]util.DecimalRatio)
		buyingAffilsShareEOPTotal := sli.BuyingAffiliateSharesAtEOPTotal()

		sli.BuyingAffiliates.ForEach(func(afId string) bool {
			afShareBalanceAtEOP := sli.ActiveAffiliateSharesAtEOP.Get(afId)
			affiliateAdjustmentPortions[afId] = util.DecimalRatio{
				afShareBalanceAtEOP, buyingAffilsShareEOPTotal}
			return true
		})

		return &_SflRatioResultResult{
			SflRatio:                          ratio,
			AcbAdjustAffiliateRatios:          affiliateAdjustmentPortions,
			FewerRemainingSharesThanSflShares: buyingAffilsShareEOPTotal.LessThan(ratio.Numerator),
		}
	}
	return &_SflRatioResultResult{}
}

// The algorithm to use to determine automatic superficial-loss adjustment
// distribution.
type AutoSflaAlgo int

const (
	// Do not allow automatic SLFA with multiple affiliates.
	SFLA_ALGO_REQUIRE_MANUAL AutoSflaAlgo = iota
	SFLA_ALGO_REJECT_IF_ANY_REGISTERED
	SFLA_ALGO_DISTRIB_BUY_RATIOS
)

type AddTxOptions struct {
	autoSflaAlgo AutoSflaAlgo
}

// Returns a TxDelta for the Tx at txs[idx].
// Optionally, returns a new Tx if a SFLA Tx was generated to accompany
// this Tx. It is expected that that Tx be inserted into txs and evaluated next.
func AddTx(
	idx int,
	txs []*Tx,
	ptfStatuses *AffiliatePortfolioSecurityStatuses,
) (*TxDelta, []*Tx, error) {

	tx := txs[idx]
	txAffiliate := NonNilTxAffiliate(tx)
	preTxStatus := ptfStatuses.GetNextPreStatus(txAffiliate.Id())

	util.Assertf(tx.Security == preTxStatus.Security,
		"AddTx: securities do not match (%s and %s)\n", tx.Security, preTxStatus.Security)

	var totalLocalSharePrice decimal.Decimal = tx.Shares.Mul(tx.AmountPerShare).Mul(tx.TxCurrToLocalExchangeRate)

	newShareBalance := preTxStatus.ShareBalance
	newAllAffiliatesShareBalance := preTxStatus.AllAffiliatesShareBalance
	registered := tx.Affiliate != nil && tx.Affiliate.Registered()
	var newAcbTotal decimal_opt.DecimalOpt = preTxStatus.TotalAcb
	var capitalGains decimal_opt.DecimalOpt = util.Tern[decimal_opt.DecimalOpt](registered, decimal_opt.Null, decimal_opt.Zero)
	var superficialLoss decimal_opt.DecimalOpt = util.Tern[decimal_opt.DecimalOpt](registered, decimal_opt.Null, decimal_opt.Zero)
	superficialLossRatio := util.DecimalRatio{}
	potentiallyOverAppliedSfl := false
	var newTxs []*Tx = nil

	// Sanity checks
	sanityCheckError := func(fmtStr string, v ...interface{}) error {
		return fmt.Errorf(
			"In transaction on %v of %d shares of %s, "+fmtStr,
			append([]interface{}{tx.TradeDate, tx.Shares, tx.Security}, v...)...)
	}
	if preTxStatus.AllAffiliatesShareBalance.LessThan(preTxStatus.ShareBalance) {
		return nil, nil, sanityCheckError("the share balance across all affiliates "+
			"(%d) is lower than the share balance for the affiliate of the sale (%d)",
			preTxStatus.AllAffiliatesShareBalance, preTxStatus.ShareBalance)
	} else if registered && !preTxStatus.TotalAcb.IsNull {
		return nil, nil, sanityCheckError("found an ACB on a registered affiliate")
	} else if !registered && preTxStatus.TotalAcb.IsNull {
		return nil, nil, sanityCheckError("found an invalid ACB (null)")
	}

	switch tx.Action {
	case BUY:
		newShareBalance = preTxStatus.ShareBalance.Add(tx.Shares)
		newAllAffiliatesShareBalance = preTxStatus.AllAffiliatesShareBalance.Add(tx.Shares)
		totalPrice := totalLocalSharePrice.Add(tx.Commission.Mul(tx.CommissionCurrToLocalExchangeRate))
		newAcbTotal = preTxStatus.TotalAcb.AddD(totalPrice)
	case SELL:
		if tx.Shares.GreaterThan(preTxStatus.ShareBalance) {
			return nil, nil, fmt.Errorf(
				"Sell order on %v of %d shares of %s is more than the current holdings (%d)",
				tx.TradeDate, tx.Shares.String(), tx.Security, preTxStatus.ShareBalance.String())
		}
		newShareBalance = preTxStatus.ShareBalance.Sub(tx.Shares)
		newAllAffiliatesShareBalance = preTxStatus.AllAffiliatesShareBalance.Sub(tx.Shares)
		// Note commission plays no effect on sell order ACB
		newAcbTotal = preTxStatus.TotalAcb.Sub(preTxStatus.PerShareAcb().MulD(tx.Shares))
		totalPayout := totalLocalSharePrice.Sub(tx.Commission.Mul(tx.CommissionCurrToLocalExchangeRate))
		capitalGains = decimal_opt.New(totalPayout).Sub(preTxStatus.PerShareAcb().MulD(tx.Shares))
		log.Tracef(traceTag, "AddTx newAcbTotal: %v, totalPayout: %v, capGain (pre registered loss adjust): %v",
			newAcbTotal, totalPayout, capitalGains)

		if !registered && capitalGains.IsNegative() {
			sflRatioResult := getSuperficialLossRatio(idx, txs, ptfStatuses)
			superficialLossRatio = sflRatioResult.SflRatio
			calculatedSuperficialLoss := decimal_opt.Zero
			if superficialLossRatio.Valid() {
				calculatedSuperficialLoss = capitalGains.MulD(superficialLossRatio.ToDecimal())
			}

			if tx.SpecifiedSuperficialLoss.Present() {
				superficialLoss = tx.SpecifiedSuperficialLoss.MustGet().SuperficialLoss
				capitalGains = capitalGains.Sub(superficialLoss)

				if !tx.SpecifiedSuperficialLoss.MustGet().Force {
					sflDiff := decimal_opt.Abs(calculatedSuperficialLoss.Sub(superficialLoss))
					var maxDiff decimal_opt.DecimalOpt = decimal_opt.NewFromFloat(0.001)
					if sflDiff.GreaterThan(maxDiff) {
						return nil, nil, fmt.Errorf(
							"Sell order on %v of %s: superficial loss was specified, but "+
								"the difference between the specified value (%f) and the "+
								"computed value (%f) is greater than the max allowed "+
								"discrepancy (%f).\nTo force this SFL value, append an '!' "+
								"to the value",
							tx.TradeDate, tx.Security, superficialLoss.String(),
							calculatedSuperficialLoss.String(), maxDiff.String())
					}
				}

				// ACB adjustment TX must be specified manually in this case.
			} else if superficialLossRatio.Valid() {
				util.Assert(sflRatioResult.AcbAdjustAffiliateRatios != nil,
					"addTx: sflRatioResult.AcbAdjustAffiliateRatios was nil")
				util.Assert(len(sflRatioResult.AcbAdjustAffiliateRatios) > 0,
					"addTx: sflRatioResult.AcbAdjustAffiliateRatios was empty")

				superficialLoss = calculatedSuperficialLoss
				capitalGains = capitalGains.Sub(calculatedSuperficialLoss)
				potentiallyOverAppliedSfl = sflRatioResult.FewerRemainingSharesThanSflShares

				acbAdjustAffiliates := util.MapKeys[string, util.DecimalRatio](sflRatioResult.AcbAdjustAffiliateRatios)
				sort.Strings(acbAdjustAffiliates)
				for _, afId := range acbAdjustAffiliates {
					ratioOfSfl := sflRatioResult.AcbAdjustAffiliateRatios[afId]
					autoAdjustAffiliate := GlobalAffiliateDedupTable.MustGet(afId)
					if ratioOfSfl.Valid() && !autoAdjustAffiliate.Registered() {
						// This new Tx will adjust (increase) the ACB for this superficial loss.
						newTxs = append(newTxs, &Tx{
							Security:                  tx.Security,
							TradeDate:                 tx.TradeDate,
							SettlementDate:            tx.SettlementDate,
							Action:                    SFLA,
							Shares:                    decimal.NewFromInt(1),
							AmountPerShare:            decimal.NewFromFloat(-1.0).Mul(superficialLoss.Decimal).Mul(ratioOfSfl.ToDecimal()),
							TxCurrency:                CAD,
							TxCurrToLocalExchangeRate: decimal.NewFromInt(1),
							Memo: fmt.Sprintf(
								"Automatic SfL ACB adjustment. %s%% (%s/%s) of SfL, which was %s/%s of sale shares.",
								ratioOfSfl.ToDecimal().Mul(decimal.NewFromInt(100)).StringFixed(2),
								ratioOfSfl.Numerator.String(), ratioOfSfl.Denominator.String(),
								superficialLossRatio.Numerator.String(), superficialLossRatio.Denominator.String(),
							),
							Affiliate: autoAdjustAffiliate,
						})
					}
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
		if !tx.Shares.IsZero() {
			return nil, nil, fmt.Errorf("Invalid RoC tx on %v: # of shares is non-zero (%d)",
				tx.TradeDate, tx.Shares.String())
		}
		acbReduction := tx.AmountPerShare.Mul(preTxStatus.ShareBalance).Mul(tx.TxCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb.SubD(acbReduction)
		if newAcbTotal.IsNegative() {
			return nil, nil, fmt.Errorf("Invalid RoC tx on %v: RoC (%f) exceeds the current ACB (%f)",
				tx.TradeDate, acbReduction.String(), preTxStatus.TotalAcb.String())
		}
	case SFLA:
		if registered {
			return nil, nil, fmt.Errorf(
				"Invalid SfLA tx on %v: Registered affiliates do not have an ACB to adjust",
				tx.TradeDate)
		}
		acbAdjustment := tx.AmountPerShare.Mul(tx.Shares).Mul(tx.TxCurrToLocalExchangeRate)
		newAcbTotal = preTxStatus.TotalAcb.AddD(acbAdjustment)
		if !(tx.TxCurrency == CAD || tx.TxCurrency == DEFAULT_CURRENCY) ||
			!tx.TxCurrToLocalExchangeRate.Equal(decimal.NewFromInt(1)) {
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
		Tx:                        tx,
		PreStatus:                 preTxStatus,
		PostStatus:                newStatus,
		CapitalGain:               capitalGains,
		SuperficialLoss:           superficialLoss,
		SuperficialLossRatio:      superficialLossRatio,
		PotentiallyOverAppliedSfl: potentiallyOverAppliedSfl,
	}
	return delta, newTxs, nil
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

	ptfStatuses := NewAffiliatePortfolioSecurityStatuses(
		txs[0].Security, initialStatus)

	for i := 0; i < len(activeTxs); i++ {
		txAffiliate := NonNilTxAffiliate(activeTxs[i])
		delta, newTxs, err := AddTx(i, activeTxs, ptfStatuses)
		if err != nil {
			// Return what we've managed so far, for debugging
			return deltas, err
		}
		log.Tracef(traceTag, "TxsToDeltaList adding PostStatus for delta: %v", delta)
		ptfStatuses.SetLatestPostStatus(txAffiliate.Id(), delta.PostStatus)
		deltas = append(deltas, delta)
		if newTxs != nil {
			// Add new Tx into modifiedTxs
			if modifiedTxs == nil {
				// Copy Txs, as we now need to modify
				modifiedTxs = make([]*Tx, 0, len(txs))
				modifiedTxs = append(modifiedTxs, txs...)
				activeTxs = modifiedTxs
			}
			// Insert into modifiedTxs after the current Tx
			for newTxI, newTx := range newTxs {
				modifiedTxs = insertTx(modifiedTxs, newTx, i+newTxI+1)
			}
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
