// Most of this file is only meant to be accessible by
// other parts of the bookkeeping module.

use std::collections::{HashMap, HashSet};

use time::{Date, Duration};

use crate::{
    portfolio::{Affiliate, Tx, TxAction, TxActionSpecifics},
    util::{
        decimal::{GreaterEqualZeroDecimal, PosDecimal},
        math::{GezDecimalRatio, PosDecimalRatio},
    },
};

use super::AffiliatePortfolioSecurityStatuses;

type Error = String;

// SplAdj : Split adjusted - Share counts may (and must) be adjusted to be in terms
//                           of the same split "period" as the sell action in
//                           question.
#[derive(PartialEq, Clone, Debug)]
struct SuperficialLossInfo {
    pub _first_date_in_period: Date,
    pub _last_date_in_period: Date,
    // Split adjusted to the time of the sale.
    pub all_aff_spladj_shares_at_end_of_period: PosDecimal,
    // This is not net, only total bought. They are split-adjusted to the time of
    // the sfl sale.
    pub total_aquired_spladj_shares_in_period: PosDecimal,
    pub buying_affiliates: HashSet<Affiliate>,
    // eop -> end of period.
    // 'Active' may include sells, so is a superset of buying_affiliates.
    // Note that right now, we technically don't actually need non-buyers
    // in here, but because we have to account for sell txs of buyers,
    // we end up inserting only-sellers simply because we don't know they
    // won't buy at some point, while we're populating.
    // We just don't bother filtering them out at the end.
    pub active_affiliate_spladj_shares_at_eop:
        HashMap<Affiliate, GreaterEqualZeroDecimal>,
}

impl SuperficialLossInfo {
    // Note: it is possible for this to legally return zero, since you could
    // have only shares remaining in non-buying affiliates.
    pub fn buying_affiliate_split_adjusted_shares_at_eop_total(
        &self,
    ) -> GreaterEqualZeroDecimal {
        let zero = GreaterEqualZeroDecimal::zero();
        let mut total = GreaterEqualZeroDecimal::zero();
        for af in &self.buying_affiliates {
            total +=
                *self.active_affiliate_spladj_shares_at_eop.get(af).unwrap_or(&zero);
        }
        total
    }
}

#[derive(Debug)]
enum MaybeSuperficialLossInfo {
    Superficial(SuperficialLossInfo),
    NotSuperficial(),
}

impl MaybeSuperficialLossInfo {
    #[cfg(test)]
    pub fn is_superficial(&self) -> bool {
        match self {
            MaybeSuperficialLossInfo::Superficial(_) => true,
            MaybeSuperficialLossInfo::NotSuperficial() => false,
        }
    }

    #[cfg(test)]
    pub fn info(&self) -> Result<&SuperficialLossInfo, ()> {
        match self {
            MaybeSuperficialLossInfo::Superficial(i) => Ok(i),
            MaybeSuperficialLossInfo::NotSuperficial() => Err(()),
        }
    }
}

pub fn get_first_day_in_superficial_loss_period(settlement_date: Date) -> Date {
    settlement_date.saturating_sub(Duration::days(30))
}

pub fn get_last_day_in_superficial_loss_period(settlement_date: Date) -> Date {
    settlement_date.saturating_add(Duration::days(30))
}

/// Checks if there is a Buy action within 30 days before or after the Sell
/// at idx, AND if you hold shares after the 30 day period
/// Also gathers relevant information for partial superficial loss calculation.
///
/// This WILL NOT consider any explicit SFL info appended to the Tx. This is only
/// for automatic detection and calculation.
/// This will also NOT check if the sale is a loss. It assumes the Sell at idx is a loss.
fn get_superficial_loss_info(
    idx: usize,
    txs: &Vec<Tx>,
    ptf_statuses: &AffiliatePortfolioSecurityStatuses,
) -> Result<MaybeSuperficialLossInfo, Error> {
    let tx = txs.get(idx).unwrap();
    assert_eq!(tx.action(), TxAction::Sell);
    let sell_shares = tx.sell_specifics().unwrap().shares;

    let first_bad_buy_date =
        get_first_day_in_superficial_loss_period(tx.settlement_date);
    let last_bad_buy_date =
        get_last_day_in_superficial_loss_period(tx.settlement_date);

    let latest_post_status = ptf_statuses.get_latest_post_status();

    // The latest post status for the selling affiliate is not yet
    // saved, so recompute the post-sale share balances.
    let all_affiliates_share_balance_after_sell =
        GreaterEqualZeroDecimal::try_from(
            *latest_post_status.all_affiliate_share_balance -
            *tx.sell_specifics().unwrap().shares
        ).map_err(|_| format!(
                "latest share balance total for all affiliates ({}) is less than sold shares ({})",
                *latest_post_status.all_affiliate_share_balance,
                tx.sell_specifics().unwrap().shares
            )
        )?;

    // Default to post-sale share balance for the affiliate
    let default_post_sale_share_balance =
        |af: &Affiliate| -> GreaterEqualZeroDecimal {
            match ptf_statuses.get_latest_post_status_for_affiliate(af) {
                Some(st) => st.share_balance,
                None => GreaterEqualZeroDecimal::zero(),
            }
        };

    // This will need to be per affiliate until stock split TXs are global
    let mut af_split_adjustments = HashMap::<&Affiliate, PosDecimal>::new();

    let mut all_aff_spladj_shares_at_end_of_period =
        all_affiliates_share_balance_after_sell;
    let mut total_aquired_spladj_shares_in_period = GreaterEqualZeroDecimal::zero();

    let mut buying_affiliates = HashSet::new();
    let mut active_affiliate_spladj_shares_at_eop =
        HashMap::<Affiliate, GreaterEqualZeroDecimal>::new();

    let sell_affiliate_share_balance_before_sell =
        default_post_sale_share_balance(&tx.affiliate);
    active_affiliate_spladj_shares_at_eop.insert(
        tx.affiliate.clone(),
        GreaterEqualZeroDecimal::try_from(*sell_affiliate_share_balance_before_sell - *sell_shares)
            .map_err(|_| format!(
                "latest share balance ({}) for affiliate ({}) is less than sold shares ({})",
                sell_affiliate_share_balance_before_sell, tx.affiliate.name(), sell_shares)
            )?
    );

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
    //  BUY 5             BUY 0
    //  ...               ...
    //  SELL 4 (SFL)      BUY 5
    //                    SELL 3
    // (reminaing: 1)     (remaining: 2)
    // use 2 or 3 as remaining shares, since it is the min val for proportional SFL.
    //
    // However, the safer thing to do might be to use the max shares, but require
    // manual entry if the number of shares remaining in the sell affiliate is less
    // than the number of rejected loss shares. <<<<<< Warn of this and possibly suggest an accountant.

    for i in (idx + 1)..txs.len() {
        let after_tx = txs.get(i).unwrap();
        if after_tx.settlement_date > last_bad_buy_date {
            break;
        }
        let after_tx_affil = &after_tx.affiliate;
        let split_adjustment: PosDecimal = af_split_adjustments
            .get(after_tx_affil)
            .map(|v| *v)
            .unwrap_or(PosDecimal::one());

        // Within the 30 day window after
        match &after_tx.action_specifics {
            TxActionSpecifics::Buy(buy) => {
                let after_tx_buy_shares = GreaterEqualZeroDecimal::from(buy.shares);
                let after_tx_buy_spladj_shares =
                    after_tx_buy_shares * split_adjustment.into();

                all_aff_spladj_shares_at_end_of_period += after_tx_buy_spladj_shares;
                let old_shares_eop = active_affiliate_spladj_shares_at_eop
                    .get(after_tx_affil)
                    .map(|d| *d)
                    .unwrap_or_else(|| {
                        default_post_sale_share_balance(after_tx_affil)
                    });
                active_affiliate_spladj_shares_at_eop.insert(
                    after_tx_affil.clone(),
                    old_shares_eop + after_tx_buy_spladj_shares,
                );
                total_aquired_spladj_shares_in_period += after_tx_buy_spladj_shares;
                buying_affiliates.insert(after_tx_affil.clone());
            }
            TxActionSpecifics::Sell(sell) => {
                let after_tx_sell_shares =
                    GreaterEqualZeroDecimal::from(sell.shares);
                let after_tx_spladj_sell_shares =
                    after_tx_sell_shares * split_adjustment.into();

                all_aff_spladj_shares_at_end_of_period = GreaterEqualZeroDecimal::try_from(
                    *all_aff_spladj_shares_at_end_of_period - *after_tx_spladj_sell_shares
                ).map_err(|_| {
                    // The caller may not have gone through those transactions to validate them
                    // yet, so we shouldn't panic here.
                    format!("Total share count went below zero in 30-day period after sale (on {})",
                            after_tx.trade_date)
                })?;

                let old_shares_eop = active_affiliate_spladj_shares_at_eop
                    .get(after_tx_affil)
                    .map(|d| *d)
                    .unwrap_or_else(|| {
                        default_post_sale_share_balance(after_tx_affil)
                    });
                active_affiliate_spladj_shares_at_eop.insert(after_tx_affil.clone(),
                    GreaterEqualZeroDecimal::try_from(*old_shares_eop - *after_tx_spladj_sell_shares)
                        .map_err(|_|
                            format!("Share count for affiliate {} went below zero in 30-day period after sale (on {})",
                            after_tx_affil.name(), after_tx.trade_date))?
                );
            }
            TxActionSpecifics::Split(split) => {
                // Adjustment goes backwards in time for txs after the sale.
                let new_split_adjustment =
                    split_adjustment / split.ratio.pre_to_post_factor();
                af_split_adjustments.insert(after_tx_affil, new_split_adjustment);
            }
            // These don't change the share quantity, so they can be ignored
            TxActionSpecifics::Roc(_)
            | TxActionSpecifics::RiCGDist(_)
            | TxActionSpecifics::RiDiv(_)
            | TxActionSpecifics::CGDiv(_)
            | TxActionSpecifics::Sfla(_) => (),
        }
    }

    // Convert end-of-period shares to PosDecimal, or declare non-superficial
    // and return.
    let all_aff_spladj_shares_at_end_of_period = if let Ok(v) =
        PosDecimal::try_from(*all_aff_spladj_shares_at_end_of_period)
    {
        v
    } else {
        // all_aff_spladj_shares_at_end_of_period was zero
        return Ok(MaybeSuperficialLossInfo::NotSuperficial());
    };

    let mut af_split_adjustments = HashMap::<&Affiliate, PosDecimal>::new();

    // Start just before the sell tx and work backwards
    for i in (0..idx).rev() {
        let before_tx = txs.get(i).unwrap();
        if before_tx.settlement_date < first_bad_buy_date {
            break;
        }
        let before_tx_affil = &before_tx.affiliate;

        let split_adjustment: PosDecimal = af_split_adjustments
            .get(before_tx_affil)
            .map(|v| *v)
            .unwrap_or(PosDecimal::one());

        // Within the 30 day window before
        match &before_tx.action_specifics {
            TxActionSpecifics::Buy(buy) => {
                let spladj_shares = buy.shares * split_adjustment;
                total_aquired_spladj_shares_in_period +=
                    GreaterEqualZeroDecimal::from(spladj_shares);
                buying_affiliates.insert(before_tx_affil.clone());

                if !active_affiliate_spladj_shares_at_eop
                    .contains_key(before_tx_affil)
                {
                    // This affiliate only bought before the superficial loss tx,
                    // so just populate them with their current status.
                    active_affiliate_spladj_shares_at_eop.insert(
                        before_tx_affil.clone(),
                        default_post_sale_share_balance(before_tx_affil),
                    );
                }
            }
            TxActionSpecifics::Split(split) => {
                // Adjustment goes forwards in time for txs before the sale.
                let new_split_adjustment =
                    split_adjustment * split.ratio.pre_to_post_factor();
                af_split_adjustments.insert(before_tx_affil, new_split_adjustment);
            }
            // ignored
            TxActionSpecifics::Sell(_)
            | TxActionSpecifics::Roc(_)
            | TxActionSpecifics::RiCGDist(_)
            | TxActionSpecifics::RiDiv(_)
            | TxActionSpecifics::CGDiv(_)
            | TxActionSpecifics::Sfla(_) => (),
        }
    }

    let x = if let Ok(bought_spladj_shares_in_period) =
        PosDecimal::try_from(*total_aquired_spladj_shares_in_period)
    {
        MaybeSuperficialLossInfo::Superficial(SuperficialLossInfo {
            _first_date_in_period: first_bad_buy_date,
            _last_date_in_period: last_bad_buy_date,
            all_aff_spladj_shares_at_end_of_period,
            total_aquired_spladj_shares_in_period: bought_spladj_shares_in_period,
            buying_affiliates: buying_affiliates,
            active_affiliate_spladj_shares_at_eop,
        })
    } else {
        MaybeSuperficialLossInfo::NotSuperficial()
    };
    Ok(x)
}

#[derive(PartialEq, Debug)]
pub(super) struct SflRatioResultResult {
    pub sfl_ratio: PosDecimalRatio,
    pub acb_adjust_affiliate_ratios: HashMap<Affiliate, GezDecimalRatio>,
    // ** Notes/warnings to emit later. **
    // Set when the sum of remaining involved affiliate shares is fewer than
    // the SFL shares, which means that the selling affiliate probably had some
    // shares they didn't sell. This can happen because we use interpretation/algo I.1
    // rather than I.2 (see the sfl wiki page) to determine the loss ratio.
    pub fewer_remaining_shares_than_sfl_shares: bool,
}

/// Calculation of partial superficial losses where
/// Superficial loss = (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)
/// This function returns the left hand side of this formula, on the condition that
/// the loss is actually superficial.
///
/// Returns:
/// - the superficial loss ratio (if calculable)
/// - the affiliate to apply an automatic adjustment to (if possible)
/// - an soft error (warning), which only applies when auto-generating the SfLA
///
/// Uses interpretation I.1 from the link below for splitting loss adjustments.
///
/// More detailed discussions about adjustment allocation can be found at
/// https://github.com/tsiemens/acb/wiki/Superficial-Losses
///
/// Reference: https://www.adjustedcostbase.ca/blog/applying-the-superficial-loss-rule-for-a-partial-disposition-of-shares/
fn calc_superficial_loss_ratio(
    sell_tx: &Tx,
    msli: MaybeSuperficialLossInfo,
) -> Result<Option<SflRatioResultResult>, Error> {
    match msli {
        MaybeSuperficialLossInfo::Superficial(sli) => {
            let sell_shares = sell_tx.sell_specifics().unwrap().shares;

            let numerator = crate::util::decimal::constrained_min(&[
                sell_shares,
                sli.total_aquired_spladj_shares_in_period,
                sli.all_aff_spladj_shares_at_end_of_period,
            ]);
            let ratio = PosDecimalRatio {
                numerator: numerator,
                denominator: sell_shares,
            };

            assert_ne!(sli.buying_affiliates.len(), 0,
                "get_superficial_loss_ratio: loss was superficial, but no buying affiliates");

            // Affiliate to percentage of the SFL adjustment is attributed to it.
            let mut affiliate_adjustment_portions = HashMap::new();
            let buying_affils_spladj_share_eop_total =
                sli.buying_affiliate_split_adjusted_shares_at_eop_total();
            // Add in ACB adjustments for the buying affiliates, if any of them have
            // shares remaining.
            // If none have shares remaining, then we are in the case where we set
            // fewer_remaining_shares_than_sfl_shares below, and it will be reported
            // as a warning. Note that that case can still occur even if we do have
            // some shares remaining on the buyers.
            if let Ok(positive_buying_affils_share_eop_total) =
                PosDecimal::try_from(*buying_affils_spladj_share_eop_total)
            {
                for af in &sli.buying_affiliates {
                    let af_spladj_share_balance_at_eop =
                        sli.active_affiliate_spladj_shares_at_eop.get(af).unwrap();
                    let af_portion = GezDecimalRatio {
                        numerator: af_spladj_share_balance_at_eop.clone(),
                        denominator: positive_buying_affils_share_eop_total,
                    };
                    affiliate_adjustment_portions.insert(af.clone(), af_portion);
                }
            }

            let affected_sfl_shares = ratio.numerator;
            Ok(Some(SflRatioResultResult {
                sfl_ratio: ratio,
                acb_adjust_affiliate_ratios: affiliate_adjustment_portions,
                fewer_remaining_shares_than_sfl_shares:
                    *buying_affils_spladj_share_eop_total < *affected_sfl_shares,
            }))
        }
        MaybeSuperficialLossInfo::NotSuperficial() => Ok(None),
    }
}

/// See doc for `calc_superficial_loss_ratio`
pub(super) fn get_superficial_loss_ratio(
    idx: usize,
    txs: &Vec<Tx>,
    ptf_statuses: &AffiliatePortfolioSecurityStatuses,
) -> Result<Option<SflRatioResultResult>, Error> {
    let msli = get_superficial_loss_info(idx, txs, ptf_statuses)?;
    calc_superficial_loss_ratio(txs.get(idx).unwrap(), msli)
}

// MARK: Tests
#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use rust_decimal_macros::dec;

    use crate::portfolio::bookkeeping::superficial_loss::SuperficialLossInfo;
    use crate::portfolio::bookkeeping::testlib::TPSS;
    use crate::portfolio::testlib::{default_sec, mk_date, TTx};

    use crate::portfolio::{
        bookkeeping::AffiliatePortfolioSecurityStatuses, Affiliate,
    };
    use crate::portfolio::{SFLInput, SplitRatio, TxAction as A};
    use crate::testlib::assert_big_struct_eq;
    use crate::util::decimal::{
        GreaterEqualZeroDecimal, LessEqualZeroDecimal, PosDecimal,
    };
    use crate::util::math::{GezDecimalRatio, PosDecimalRatio};
    use crate::{gezdec as gez, pdec};

    use super::{
        calc_superficial_loss_ratio, get_superficial_loss_info,
        MaybeSuperficialLossInfo, SflRatioResultResult,
    };

    fn create_test_status(
        af_shares: &[(Affiliate, GreaterEqualZeroDecimal)],
    ) -> AffiliatePortfolioSecurityStatuses {
        let mut statuses = AffiliatePortfolioSecurityStatuses::new(default_sec());
        let mut total = GreaterEqualZeroDecimal::zero();
        for (af, shares) in af_shares {
            total += *shares;
            let acb = if af.registered() { None } else { Some(gez!(1)) };
            statuses.set_latest_post_status(
                af,
                TPSS {
                    shares: shares.clone(),
                    all_shares: total.clone(),
                    acb_per_sh: acb,
                    ..TPSS::d()
                }
                .x(),
            )
        }
        statuses
    }

    // MARK: get_superficial_loss_info tests

    // Non-superficial:
    //  - zero shares at end of period
    //      - sell all in two fractional shares (decimal) txs
    //  - no buys within period
    //  - tx is only in array
    //  - explicit SFLA is ignored
    //
    // Note that this function doesn't check if the sell is actually a loss.
    #[test]
    #[rustfmt::skip]
    fn test_get_superficial_loss_info_non_superficial() {
        let default_af = || Affiliate::default();
        let af_br = ||Affiliate::from_strep("B(R)");

        let ignored_explicit_sfl = SFLInput{
            superficial_loss: LessEqualZeroDecimal::try_from(dec!(-1)).unwrap(),
            force: true };

        // Case: Zero shares at end of period
        // - Includes fractional share check
        let statuses = create_test_status(
            &[(default_af(), gez!(5)), (af_br(), gez!(10))]);

        let txs = vec![
            // Share quantity of buys before are ignored
            TTx{t_day: 10, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
            TTx{t_day: 10, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 11, act: A::Sell, shares: gez!(7), price: gez!(0.00001),
                af: af_br(),
                sfl: Some(ignored_explicit_sfl.clone()), ..TTx::d()}.x(),
            // Buy after
            TTx{t_day: 12, act: A::Buy, shares: gez!(1.2), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
            TTx{t_day: 13, act: A::Sell, shares: gez!(0.7), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
            // Disposal of existing shares
            TTx{t_day: 41, act: A::Sell, shares: gez!(5), price: gez!(0.5),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 41, act: A::Sell, shares: gez!(3.5), price: gez!(0.5),
                af: af_br(), ..TTx::d()}.x(),
            // Out of range, ignored.
            TTx{t_day: 42, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(2, &txs, &statuses).unwrap();
        assert!(!minfo.is_superficial());

        // Case: Sell is only tx, and sells all shares
        let statuses = create_test_status(&[(default_af(), gez!(5))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 11, act: A::Sell, shares: gez!(5), price: gez!(0.0001),
                af: default_af(),
                sfl: Some(ignored_explicit_sfl.clone()), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(0, &txs, &statuses).unwrap();
        assert!(!minfo.is_superficial());

        // Case: No buys within period.
        let statuses = create_test_status(
            &[(default_af(), gez!(5)), (af_br(), gez!(10))]);

        let txs = vec![
            // Share quantity of buys before are ignored
            TTx{t_day: 10, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
            TTx{t_day: 10, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 41, act: A::Sell, shares: gez!(7), price: gez!(0.0001),
                af: af_br(),
                sfl: Some(ignored_explicit_sfl.clone()), ..TTx::d()}.x(),
            // Other sells after (small)
            TTx{t_day: 42, act: A::Sell, shares: gez!(1), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
            // Out of range, ignored.
            TTx{t_day: 72, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(2, &txs, &statuses).unwrap();
        assert!(!minfo.is_superficial());
    }

    // Superficial:
    // - buy before
    //     - tx is last in array
    // - buy after
    //     - tx is first in array
    // - buys both before and after
    //     - other and selling affil
    #[test]
    #[rustfmt::skip]
    fn test_get_superficial_loss_info_superficial_basic() {
        let default_af = || Affiliate::default();
        let af_br = ||Affiliate::from_strep("B(R)");
        let af_c = ||Affiliate::from_strep("C");

        // Case: Sells only before, and tx at end of vector
        let statuses = create_test_status(
            &[(default_af(), gez!(5)), (af_br(), gez!(10))]);

        let txs = vec![
            // Share quantity of buys before are ignored
            TTx{t_day: 0, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // First in period
            TTx{t_day: 1, act: A::Buy, shares: gez!(77), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(1), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(7), price: gez!(10000),
                af: af_br(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(3, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            // Account for settlement date offset of 2 days
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            all_aff_spladj_shares_at_end_of_period: pdec!(8),
            total_aquired_spladj_shares_in_period: pdec!(78),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(5)),
                (af_br(), gez!(3)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Sells only after, and tx at start of vector
        let statuses = create_test_status(
            &[(af_br(), gez!(10)), (default_af(), gez!(5))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 1, act: A::Sell, shares: gez!(7), price: gez!(10000),
                af: af_br(), ..TTx::d()}.x(),
            // Buys after
            TTx{t_day: 31, act: A::Buy, shares: gez!(77), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // AF not in pre-status
            TTx{t_day: 31, act: A::Buy, shares: gez!(5), price: gez!(1),
                af: af_c(), ..TTx::d()}.x(),
            // After period
            TTx{t_day: 32, act: A::Buy, shares: gez!(33), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(0, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            // Account for settlement date offset of 2 days
            _first_date_in_period: mk_date(1+2-30),
            _last_date_in_period: mk_date(1+2+30),
            all_aff_spladj_shares_at_end_of_period: pdec!(90),
            total_aquired_spladj_shares_in_period: pdec!(82),
            buying_affiliates: HashSet::from([default_af(), af_c()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(82)),
                (af_br(), gez!(3)),
                (af_c(), gez!(5)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Sells before and after
        //  also covers:
        //  - selling-only affiliates (C)
        //  - explicit SFLA Tx is ignored (shares)
        //  - sells before go below zero, but are ignored as invalid.
        let statuses = create_test_status(
            &[(af_br(), gez!(10)), (default_af(), gez!(5)), (af_c(), gez!(7))]);

        let txs = vec![
            // First in period
            TTx{t_day: 15, act: A::Buy, shares: gez!(1), price: gez!(1),
                af: af_br(), ..TTx::d()}.x(),
            // This is technically invalid (sell below zero), but we ignore
            // sells before.
            TTx{t_day: 15, act: A::Sell, shares: gez!(77), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 40, act: A::Sell, shares: gez!(7), price: gez!(10000),
                af: af_br(), ..TTx::d()}.x(),
            // Buys after
            TTx{t_day: 50, act: A::Buy, shares: gez!(77), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 51, act: A::Sell, shares: gez!(5), price: gez!(1),
                af: af_c(), ..TTx::d()}.x(),
            // Ignored Tx type
            TTx{t_day: 52, act: A::Sfla, shares: gez!(5), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // After period
            TTx{t_day: 80, act: A::Buy, shares: gez!(1), price: gez!(1),
                af: af_c(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(2, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            // Account for settlement date offset of 2 days
            _first_date_in_period: mk_date(40+2-30),
            _last_date_in_period: mk_date(40+2+30),
            all_aff_spladj_shares_at_end_of_period: pdec!(87),
            total_aquired_spladj_shares_in_period: pdec!(78),
            buying_affiliates: HashSet::from([default_af(), af_br()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(82)),
                (af_br(), gez!(3)),
                (af_c(), gez!(2)),
            ]),
        };
        assert_big_struct_eq(info, &expected);
    }

    // Superficial:
    // - all buying affiliates have zero shares at end
    // - sell all and repurchase some within period
    #[test]
    #[rustfmt::skip]
    fn test_get_superficial_loss_info_superficial_special() {
        let default_af = || Affiliate::default();
        let af_b = ||Affiliate::from_strep("B");

        // Case: All buying affiliates have zero shares at end
        //
        // AF Default: Owns 1
        // wait...
        // AF B: Buy 10
        // AF B: Sell 10 (superficial)
        let statuses = create_test_status(
            &[(af_b(), gez!(10)), (default_af(), gez!(1))]);

        let txs = vec![
            TTx{t_day: 10, act: A::Buy, shares: gez!(10), price: gez!(1),
                af: af_b(), ..TTx::d()}.x(),
            // Superficial
            TTx{t_day: 11, act: A::Sell, shares: gez!(10), price: gez!(0.5),
                af: af_b(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(1, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            // Account for settlement date offset of 2 days
            _first_date_in_period: mk_date(11+2-30),
            _last_date_in_period: mk_date(11+2+30),
            all_aff_spladj_shares_at_end_of_period: pdec!(1),
            total_aquired_spladj_shares_in_period: pdec!(10),
            buying_affiliates: HashSet::from([af_b()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (af_b(), gez!(0)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: sell all and repurchase some within period
        let statuses = create_test_status(&[(default_af(), gez!(10))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(10), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 11, act: A::Buy, shares: gez!(10), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 12, act: A::Sell, shares: gez!(10), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 13, act: A::Buy, shares: gez!(5), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(0, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            // Account for settlement date offset of 2 days
            _first_date_in_period: mk_date(10+2-30),
            _last_date_in_period: mk_date(10+2+30),
            all_aff_spladj_shares_at_end_of_period: pdec!(5),
            total_aquired_spladj_shares_in_period: pdec!(15),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(5)),
            ]),
        };
        assert_big_struct_eq(info, &expected);
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_superficial_loss_info_superficial_errors() {
        let default_af = || Affiliate::default();
        let af_b = ||Affiliate::from_strep("B");

        // Case: initial sell is too large
        let statuses = create_test_status(&[(default_af(), gez!(10))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(11), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
        ];
        let e = get_superficial_loss_info(0, &txs, &statuses).unwrap_err();
        assert_eq!(
            "latest share balance total for all affiliates (10) is less than sold shares (11)",
            e);

        // Case: initial sell is too much, and no status present
        let statuses = create_test_status(&[]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(11), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
        ];
        let e = get_superficial_loss_info(0, &txs, &statuses).unwrap_err();
        assert_eq!(
            "latest share balance total for all affiliates (0) is less than sold shares (11)",
            e);

        // Case: initial sell is too much, but not exceeding total shares
        let statuses = create_test_status(&[(af_b(), gez!(100))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(11), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
        ];
        let e = get_superficial_loss_info(0, &txs, &statuses).unwrap_err();
        assert_eq!(
            "latest share balance (0) for affiliate (Default) is less than sold shares (11)",
            e);

        // Case: Some sell after is too large (combined with buys, order dependent)
        let statuses = create_test_status(&[(default_af(), gez!(10))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(5), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 11, act: A::Buy, shares: gez!(3), price: gez!(1),
                af: default_af(), ..TTx::d()}.x(),
            // Buy on other AF has no effect
            TTx{t_day: 12, act: A::Buy, shares: gez!(100), price: gez!(1),
                af: af_b(), ..TTx::d()}.x(),
            // Sell is too large
            TTx{t_day: 13, act: A::Sell, shares: gez!(20), price: gez!(2),
                af: default_af(), ..TTx::d()}.x(),
        ];

        let e = get_superficial_loss_info(0, &txs, &statuses).unwrap_err();
        assert_eq!("Share count for affiliate Default went below zero in \
                    30-day period after sale (on 2017-01-14)", &e);

        // Case: Some sell after is too large, on other affiliate
        let statuses = create_test_status(&[(default_af(), gez!(10))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(5), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 11, act: A::Buy, shares: gez!(3), price: gez!(1),
                af: af_b(), ..TTx::d()}.x(),
            // Sell is too large
            TTx{t_day: 13, act: A::Sell, shares: gez!(4), price: gez!(2),
                af: af_b(), ..TTx::d()}.x(),
        ];

        let e = get_superficial_loss_info(0, &txs, &statuses).unwrap_err();
        assert_eq!("Share count for affiliate B went below zero in \
                    30-day period after sale (on 2017-01-14)", &e);

        // Case: Some sell after is too large, on other affiliate, and goes
        // below total for all affiliates
        let statuses = create_test_status(&[(default_af(), gez!(10))]);

        let txs = vec![
            // SFL candidate
            TTx{t_day: 10, act: A::Sell, shares: gez!(5), price: gez!(0.0001),
                af: default_af(), ..TTx::d()}.x(),
            TTx{t_day: 11, act: A::Buy, shares: gez!(3), price: gez!(1),
                af: af_b(), ..TTx::d()}.x(),
            // Sell is too large
            TTx{t_day: 13, act: A::Sell, shares: gez!(20), price: gez!(2),
                af: af_b(), ..TTx::d()}.x(),
        ];

        let e = get_superficial_loss_info(0, &txs, &statuses).unwrap_err();
        assert_eq!(
            "Total share count went below zero in 30-day period after sale (on 2017-01-14)",
            &e);
    }

    // MARK: get_superficial_loss_ratio / calc_superficial_loss_ratio tests

    #[test]
    #[rustfmt::skip]
    fn test_calc_superficial_loss_ratio_not_superficial() {
        let sell_tx = TTx{t_day: 10, act: A::Sell, shares: gez!(5), price: gez!(1),
                          af: Affiliate::default(), ..TTx::d()}.x();
        let msli = MaybeSuperficialLossInfo::NotSuperficial();
        let res = calc_superficial_loss_ratio(&sell_tx, msli).unwrap();
        assert!(res.is_none());
    }

    fn pratio(n: PosDecimal, d: PosDecimal) -> PosDecimalRatio {
        PosDecimalRatio {
            numerator: n,
            denominator: d,
        }
    }

    fn zratio(n: GreaterEqualZeroDecimal, d: PosDecimal) -> GezDecimalRatio {
        GezDecimalRatio {
            numerator: n,
            denominator: d,
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_calc_superficial_loss_ratio_basic_ratio() {
        let default_af = || Affiliate::default();

        // Ratio takes the min of each - sell shares, total_aquired, all_shares_eop
        //                               3            , 7            , 5

        let sell_tx = TTx{t_day: 10, act: A::Sell, shares: gez!(3), price: gez!(1),
            af: default_af(), ..TTx::d()}.x();

        let sli = SuperficialLossInfo {
            _first_date_in_period: mk_date(0),
            _last_date_in_period: mk_date(0),
            all_aff_spladj_shares_at_end_of_period: pdec!(5),
            total_aquired_spladj_shares_in_period: pdec!(7),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(6)),
            ]),
        };

        // af shares at eop / buying_affiliate_shares_at_eop_total

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(3), pdec!(3)),
                acb_adjust_affiliate_ratios: HashMap::from([
                    // af shares at eop / buying_affiliate_shares_at_eop_total
                    (default_af(), zratio(gez!(6), pdec!(6))),
                ]),
                fewer_remaining_shares_than_sfl_shares: false,
            }
        );

        // Ratio takes the min of each - sell shares, total_aquired, all_shares_eop
        //                               6            , 7            , 5

        let sell_tx = TTx{t_day: 10, act: A::Sell, shares: gez!(6), price: gez!(1),
            af: default_af(), ..TTx::d()}.x();

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(5), pdec!(6)),
                acb_adjust_affiliate_ratios: HashMap::from([
                    // af shares at eop / buying_affiliate_shares_at_eop_total
                    (default_af(), zratio(gez!(6), pdec!(6))),
                ]),
                fewer_remaining_shares_than_sfl_shares: false,
            }
        );

        // Ratio takes the min of each - sell shares, total_aquired, all_shares_eop
        //                               5            , 4            , 3

        let sell_tx = TTx{t_day: 10, act: A::Sell, shares: gez!(5), price: gez!(1),
            af: default_af(), ..TTx::d()}.x();

        let mut sli = sli;
        sli.all_aff_spladj_shares_at_end_of_period = pdec!(3);
        sli.total_aquired_spladj_shares_in_period = pdec!(4);

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(3), pdec!(5)),
                acb_adjust_affiliate_ratios: HashMap::from([
                    // af shares at eop / buying_affiliate_shares_at_eop_total
                    (default_af(), zratio(gez!(6), pdec!(6))),
                ]),
                fewer_remaining_shares_than_sfl_shares: false,
            }
        );
    }

    // calc_superficial_loss_ratio cases:
    //
    // Takes the min of each - shares, total_aquired, all_shares_eop
    // assert? no buying affiliates?
    //
    // affiliate ratios:
    // af shares at eop / BuyingAffiliateSharesAtEOPTotal
    //  - test for zero af shares
    //  - test for zero buying_affiliate_shares_at_eop_total shares
    //      (this would mean 0/0)
    //
    // FewerRemainingSharesThanSflShares : fewer_remaining_shares_than_sfl_shares
    //
    // Errors?
    // buyingAffilsShareEOPTotal is zero

    #[test]
    #[rustfmt::skip]
    fn test_calc_superficial_loss_ratio_affiliate_ratios() {
        let default_af = || Affiliate::default();
        let af_b = ||Affiliate::from_strep("B");
        let af_c = ||Affiliate::from_strep("C");

        // Active affiliate with no buys

        let sell_tx = TTx{t_day: 10, act: A::Sell, shares: gez!(3), price: gez!(1),
            af: default_af(), ..TTx::d()}.x();

        let mut sli = SuperficialLossInfo {
            _first_date_in_period: mk_date(0),
            _last_date_in_period: mk_date(0),
            all_aff_spladj_shares_at_end_of_period: pdec!(5),
            total_aquired_spladj_shares_in_period: pdec!(7),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(6)),
                (af_b(), gez!(10)),
            ]),
        };

        // af shares at eop / buying_affiliate_shares_at_eop_total

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(3), pdec!(3)),
                acb_adjust_affiliate_ratios: HashMap::from([
                    // af shares at eop / buying_affiliate_shares_at_eop_total
                    (default_af(), zratio(gez!(6), pdec!(6))),
                ]),
                fewer_remaining_shares_than_sfl_shares: false,
            }
        );

        // Distributed ratios, including zero

        sli.buying_affiliates = HashSet::from([default_af(), af_b(), af_c()]);
        sli.active_affiliate_spladj_shares_at_eop = HashMap::from([
            (default_af(), gez!(6)),
            (af_b(), gez!(10)),
            (af_c(), gez!(0)),
        ]);

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(3), pdec!(3)),
                acb_adjust_affiliate_ratios: HashMap::from([
                    // af shares at eop / buying_affiliate_shares_at_eop_total
                    (default_af(), zratio(gez!(6), pdec!(16))),
                    (af_b(), zratio(gez!(10), pdec!(16))),
                    (af_c(), zratio(gez!(0), pdec!(16))),
                ]),
                fewer_remaining_shares_than_sfl_shares: false,
            }
        );

        // Low buyer shares

        // Lower than sold shares, which is 3
        sli.buying_affiliates = HashSet::from([default_af()]);
        sli.active_affiliate_spladj_shares_at_eop = HashMap::from([
            (default_af(), gez!(2)),
        ]);

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(3), pdec!(3)),
                acb_adjust_affiliate_ratios: HashMap::from([
                    // af shares at eop / buying_affiliate_shares_at_eop_total
                    (default_af(), zratio(gez!(2), pdec!(2))),
                ]),
                // Take note: This is true now
                fewer_remaining_shares_than_sfl_shares: true,
            }
        );

        // Zero denominator (all remaining shares are in non-active or sell-only AF)

        sli.buying_affiliates = HashSet::from([default_af()]);
        sli.active_affiliate_spladj_shares_at_eop = HashMap::from([
            (default_af(), gez!(0)),
        ]);

        let res = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli.clone())).unwrap();
        assert_big_struct_eq(
            res.unwrap(),
            SflRatioResultResult {
                // min(sell_shares, total_aquired, all_shares_eop) / sell_shares
                sfl_ratio: pratio(pdec!(3), pdec!(3)),
                acb_adjust_affiliate_ratios: HashMap::from([
                ]),
                fewer_remaining_shares_than_sfl_shares: true,
            }
        );
    }

    #[test]
    #[should_panic]
    #[rustfmt::skip]
    fn test_calc_superficial_loss_ratio_affiliate_ratios_no_buying_afs() {
        let sell_tx = TTx{t_day: 10, act: A::Sell, shares: gez!(3), price: gez!(1),
            af: Affiliate::default(), ..TTx::d()}.x();

        let sli = SuperficialLossInfo {
            _first_date_in_period: mk_date(0),
            _last_date_in_period: mk_date(0),
            all_aff_spladj_shares_at_end_of_period: pdec!(5),
            total_aquired_spladj_shares_in_period: pdec!(7),
            buying_affiliates: HashSet::from([]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (Affiliate::default(), gez!(6)),
            ]),
        };

        // This will panic, because buying_affiliates was empty, even though
        // marked as superficial.
        let _ = calc_superficial_loss_ratio(
            &sell_tx, MaybeSuperficialLossInfo::Superficial(sli));
    }

    #[test]
    #[rustfmt::skip]
    fn test_calc_superficial_loss_ratio_with_stock_splits() {
        let default_af = || Affiliate::default();
        let af_r = || Affiliate::default_registered();
        let ratio = |strep| Some(SplitRatio::parse(strep).unwrap());

        // Case: Split before SFL with preceding buys and sell

        // 100*2 + (71*2) - (5*2) + 1 - 5 = 328
        let statuses = create_test_status(&[(default_af(), gez!(328))]);

        let txs = vec![
            // Share quantity of buys before are ignored
            TTx{t_day: 0, act: A::Buy, shares: gez!(100), price: gez!(1), ..TTx::d()}.x(),
            // First in period
            TTx{t_day: 1, act: A::Buy, shares: gez!(71), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 1, act: A::Sell, shares: gez!(5), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 3, act: A::Split, split: ratio("2-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 4, act: A::Buy, shares: gez!(1), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 5, act: A::Sell, shares: gez!(5), price: gez!(1), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(7), price: gez!(10000), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(6, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            // Account for settlement date offset of 2 days
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            // 100*2 + (71*2) - (5*2) + 1 - 5 - 7 = 321
            all_aff_spladj_shares_at_end_of_period: pdec!(321),
            // (71*2) + 1 = 143
            total_aquired_spladj_shares_in_period: pdec!(143),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(321)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Split before SFL where preceding sale would go below zero otherwise

        // 100*2 - 160 = 40
        let statuses = create_test_status(&[(default_af(), gez!(40))]);

        let txs = vec![
            // First in period
            TTx{t_day: 1, act: A::Buy, shares: gez!(100), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 2, act: A::Split, split: ratio("2-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 3, act: A::Sell, shares: gez!(160), price: gez!(1), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(5), price: gez!(10000), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(3, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            // 100*2 - 160 - 5 = 35
            all_aff_spladj_shares_at_end_of_period: pdec!(35),
            total_aquired_spladj_shares_in_period: pdec!(200),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(35)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Split before SFL where preceding buy would put total below zero
        // otherwise
        // (10 + 100) / 2 = 55
        let statuses = create_test_status(&[(default_af(), gez!(55))]);

        let txs = vec![
            // First in period
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 2, act: A::Buy, shares: gez!(100), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 3, act: A::Split, split: ratio("1-for-2"), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(5), price: gez!(10000), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(3, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            // (10 + 100) / 2 - 5 = 50
            all_aff_spladj_shares_at_end_of_period: pdec!(50),
            total_aquired_spladj_shares_in_period: pdec!(55),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(50)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Split after SFL with followup buys
        let statuses = create_test_status(&[(default_af(), gez!(100))]);

        let txs = vec![
            // Share quantity of buys before are ignored
            TTx{t_day: 0, act: A::Buy, shares: gez!(100), price: gez!(1), ..TTx::d()}.x(),
            // First in period
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(5), price: gez!(10000), ..TTx::d()}.x(),

            TTx{t_day: 32, act: A::Buy, shares: gez!(10), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 33, act: A::Sell, shares: gez!(5), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 34, act: A::Split, split: ratio("2-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 35, act: A::Buy, shares: gez!(15), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 36, act: A::Sell, shares: gez!(2), price: gez!(1), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(1, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            // 100 - 5 + 10 - 5 + (15 - 2)/2 = 106.5
            all_aff_spladj_shares_at_end_of_period: pdec!(106.5),
            // 10 + (15)/2 = 17.5
            total_aquired_spladj_shares_in_period: pdec!(17.5),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(106.5)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Splits only (not superficial)

        // 100 * 2 / 5 = 40
        let statuses = create_test_status(&[(default_af(), gez!(40))]);

        let txs = vec![
            // Share quantity of buys before are ignored
            TTx{t_day: 0, act: A::Buy, shares: gez!(100), price: gez!(1), ..TTx::d()}.x(),
            // First in period
            TTx{t_day: 3, act: A::Split, split: ratio("2-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 4, act: A::Split, split: ratio("1-for-5"), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(7), price: gez!(10000), ..TTx::d()}.x(),

            TTx{t_day: 32, act: A::Split, split: ratio("3-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 32, act: A::Split, split: ratio("1-for-2"), ..TTx::d()}.x(),
        ];
        let minfo = get_superficial_loss_info(3, &txs, &statuses).unwrap();
        assert!(! minfo.is_superficial());

        // Case: Split chaos before and after
        // 10 - 4 + 1/2*( 20 - 5 + 3*(40) ) = 73.5
        let statuses = create_test_status(&[(default_af(), gez!(73.5))]);

        let txs = vec![
            // First in period
            TTx{t_day: 1, act: A::Buy, shares: gez!(40), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 2, act: A::Split, split: ratio("3-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 3, act: A::Buy, shares: gez!(20), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 4, act: A::Sell, shares: gez!(5), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 5, act: A::Split, split: ratio("1-for-2"), ..TTx::d()}.x(),
            TTx{t_day: 6, act: A::Buy, shares: gez!(10), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 7, act: A::Sell, shares: gez!(4), price: gez!(1), ..TTx::d()}.x(),
            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(6), price: gez!(10000), ..TTx::d()}.x(),

            TTx{t_day: 40, act: A::Buy, shares: gez!(30), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 41, act: A::Sell, shares: gez!(9), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 42, act: A::Split, split: ratio("3-for-1"), ..TTx::d()}.x(),
            TTx{t_day: 43, act: A::Buy, shares: gez!(15), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 44, act: A::Sell, shares: gez!(11), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 45, act: A::Split, split: ratio("1-for-2"), ..TTx::d()}.x(),
            TTx{t_day: 46, act: A::Buy, shares: gez!(50), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 47, act: A::Sell, shares: gez!(5), price: gez!(1), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(7, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            // 73.5 - 6 + 30 - 9 + 1/3*( 15 - 11 + 2*(50 - 5)) = 119.83333
            all_aff_spladj_shares_at_end_of_period: pdec!(119.83333333333333333333333333),
            // 40, 20, 10, 30, 15, 50
            // (40*3/2) + (20/2) + (10) + (30) + (15/3) + (50/3*2) = 148.333333333
            total_aquired_spladj_shares_in_period: pdec!(148.33333333333333333333333333),
            buying_affiliates: HashSet::from([default_af()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                (default_af(), gez!(119.83333333333333333333333333)),
            ]),
        };
        assert_big_struct_eq(info, &expected);

        // Case: Multi-affiliate splits (or absense of splits)

        // For now, the user needs to specify the split in all affiliates holding
        // the asset, so forgetting this will result in weird results
        // (demonstrated below)

        let statuses = create_test_status(&[
            // (39*3) = 117
            (default_af(), gez!(117)),
            (af_r(), gez!(28)),
        ]);

        let txs = vec![
            // First in period
            TTx{t_day: 1, act: A::Buy, shares: gez!(40), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 1, act: A::Sell, shares: gez!(1), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 2, act: A::Buy, shares: gez!(30), price: gez!(1),
                af: af_r(), ..TTx::d()}.x(),
            TTx{t_day: 2, act: A::Sell, shares: gez!(2), price: gez!(1),
                af: af_r(), ..TTx::d()}.x(),
            // Split only in default
            TTx{t_day: 3, act: A::Split, split: ratio("3-for-1"), ..TTx::d()}.x(),

            // SFL candidate
            TTx{t_day: 31, act: A::Sell, shares: gez!(6), price: gez!(10000), ..TTx::d()}.x(),

            // Split only in (R)
            TTx{t_day: 42, act: A::Split, split: ratio("2-for-1"), af: af_r(), ..TTx::d()}.x(),
            TTx{t_day: 43, act: A::Buy, shares: gez!(50), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 44, act: A::Sell, shares: gez!(4), price: gez!(1), ..TTx::d()}.x(),
            TTx{t_day: 45, act: A::Buy, shares: gez!(60), price: gez!(1),
                af: af_r(), ..TTx::d()}.x(),
            TTx{t_day: 46, act: A::Sell, shares: gez!(6), price: gez!(1),
                af: af_r(), ..TTx::d()}.x(),
        ];

        let minfo = get_superficial_loss_info(5, &txs, &statuses).unwrap();
        let info = minfo.info().unwrap();
        let expected = SuperficialLossInfo {
            _first_date_in_period: mk_date(31+2-30),
            _last_date_in_period: mk_date(31+2+30),
            // 117 + 28 - 6 + (50-4) + (60-6)/2 = 212
            all_aff_spladj_shares_at_end_of_period: pdec!(212),
            // (40*3) + (30) + (50) + (60/2) = 230
            total_aquired_spladj_shares_in_period: pdec!(230),
            buying_affiliates: HashSet::from([default_af(), af_r()]),
            active_affiliate_spladj_shares_at_eop: HashMap::from([
                // 117 - 6 + 50 - 4
                (default_af(), gez!(157)),
                // 28 + (60-6)/2
                (af_r(), gez!(55)),
            ]),
        };
        assert_big_struct_eq(info, &expected);
    }
}
