// Most of this file is only meant to be accessible by
// other parts of the bookkeeping module.

use std::collections::{HashMap, HashSet};

use time::{Date, Duration};

use crate::{portfolio::{Affiliate, Tx, TxAction, TxActionSpecifics}, util::{decimal::GreaterEqualZeroDecimal, math::PosDecimalRatio}};

use super::AffiliatePortfolioSecurityStatuses;

type Error = String;

struct SuperficialLossInfo {
    pub is_superficial: bool,
    pub first_date_in_period: Date,
    pub last_date_in_period: Date,
    pub all_aff_shares_at_end_of_period: GreaterEqualZeroDecimal,
    // This is not net, only total bought
    pub total_aquired_in_period: GreaterEqualZeroDecimal,
    pub buying_affiliates: HashSet<Affiliate>,
    // eop -> end of period.
    // 'Active' may include sells, so is a superset of buying_affiliates
    pub active_affiliate_shares_at_eop: HashMap<Affiliate, GreaterEqualZeroDecimal>
}

impl SuperficialLossInfo {
    pub fn buying_affiliate_shares_at_eop_total(&self) -> GreaterEqualZeroDecimal {
        let zero = GreaterEqualZeroDecimal::zero();
        let mut total = GreaterEqualZeroDecimal::zero();
        for af in &self.buying_affiliates {
            total += *self.active_affiliate_shares_at_eop.get(af).unwrap_or(&zero);
        }
        total
    }
}

fn get_first_day_in_superficial_loss_period(settlement_date: Date) -> Date {
    settlement_date.saturating_sub(Duration::days(30))
}

fn get_last_day_in_superficial_loss_period(settlement_date: Date) -> Date {
    settlement_date.saturating_add(Duration::days(30))
}

// This should maybe be in a separate util file.
fn get_or_set_default<'a, K, V, F>(map: &'a mut HashMap<K, V>, k: &K, default_fn: F) -> &'a V
    where K: Clone + Eq + PartialEq + std::hash::Hash,
          F: Fn(&K) -> V,
    {

    // This is done like so (not using match of 'if let') because the
    // borrow checker will complain if the condition/match itself takes
    // an immutable reference. Even if we scope the whole thing, it will
    // insist that the borrow lives last that scope. I'm not exactly sure why.
    // It may just be a limitation.
    if map.get(k).is_some() {
        return map.get(k).unwrap();
    }

    map.insert(k.clone(), default_fn(k));
    map.get(k).unwrap()
}

/// Checks if there is a Buy action within 30 days before or after the Sell
/// at idx, AND if you hold shares after the 30 day period
/// Also gathers relevant information for partial superficial loss calculation.
fn get_superficial_loss_info(
    idx: usize, txs: Vec<Tx>, ptf_statuses: &AffiliatePortfolioSecurityStatuses)
    -> Result<SuperficialLossInfo, Error> {

    let tx = txs.get(idx).unwrap();
    assert_eq!(tx.action(), TxAction::Sell);
    let sell_shares = tx.sell_specifics().unwrap().shares;

    let first_bad_buy_date = get_first_day_in_superficial_loss_period(tx.settlement_date);
    let last_bad_buy_date = get_last_day_in_superficial_loss_period(tx.settlement_date);

    let latest_post_status = ptf_statuses.get_latest_post_status();

    // The latest post status for the selling affiliate is not yet
    // saved, so recompute the post-sale share balances.
    let all_affiliates_share_balance_after_sell =
        GreaterEqualZeroDecimal::try_from(
            *latest_post_status.all_affiliate_share_balance -
            *tx.sell_specifics().unwrap().shares
        ).map_err(|_| format!(
                "latest all_affiliate_share_balance ({}) is less than sold shares ({})",
                *latest_post_status.all_affiliate_share_balance,
                tx.sell_specifics().unwrap().shares
            )
        )?;

    let sell_affiliate_share_balance_before_sell =
        match ptf_statuses.get_latest_post_status_for_affiliate(&tx.affiliate) {
            Some(st) => st.share_balance,
            None => GreaterEqualZeroDecimal::zero(),
        };
    let sell_affiliate_share_balance_after_sell =
        GreaterEqualZeroDecimal::try_from(
            *sell_affiliate_share_balance_before_sell - *sell_shares
        ).map_err(|_| format!(
            "latest share_balance ({}) for affiliate ({}) is less than sold shares ({})",
            sell_affiliate_share_balance_before_sell, tx.affiliate.name(), sell_shares)
        )?;

    // Default to post-sale share balance for the affiliate
    let default_post_sale_share_balance = |af: &Affiliate| -> GreaterEqualZeroDecimal {
        if af == &tx.affiliate {
            // See above. We can't directly use the most recent post-status for the
            // current tx affiliate.
            sell_affiliate_share_balance_after_sell
        } else {
            match ptf_statuses.get_latest_post_status_for_affiliate(af) {
                Some(st) => st.share_balance,
                None => GreaterEqualZeroDecimal::zero(),
            }
        }
    };

    let mut sli = SuperficialLossInfo{
        is_superficial: false,
        first_date_in_period: first_bad_buy_date,
        last_date_in_period: last_bad_buy_date,
        all_aff_shares_at_end_of_period: all_affiliates_share_balance_after_sell,
        total_aquired_in_period: GreaterEqualZeroDecimal::zero(),
        buying_affiliates: HashSet::new(),
        active_affiliate_shares_at_eop: HashMap::new(),
    };

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

    let mut did_buy_after_in_period = false;
    for i in (idx + 1)..txs.len() {
        let after_tx = txs.get(i).unwrap();
        if after_tx.settlement_date > last_bad_buy_date {
            break;
        }
        let after_tx_affil = &after_tx.affiliate;

        // Within the 30 day window after
        match &after_tx.action_specifics {
            TxActionSpecifics::Buy(buy) => {
                did_buy_after_in_period = true;
                let after_tx_buy_shares =
                    GreaterEqualZeroDecimal::from(buy.shares);
                sli.all_aff_shares_at_end_of_period += after_tx_buy_shares;
                let old_shares_eop = *get_or_set_default(
                    &mut sli.active_affiliate_shares_at_eop,
                    after_tx_affil, default_post_sale_share_balance);
                sli.active_affiliate_shares_at_eop.insert(after_tx_affil.clone(), old_shares_eop + after_tx_buy_shares);
                sli.total_aquired_in_period += after_tx_buy_shares;
                sli.buying_affiliates.insert(after_tx_affil.clone());
            },
            TxActionSpecifics::Sell(sell) => {
                let after_tx_sell_shares =
                    GreaterEqualZeroDecimal::from(sell.shares);
                    sli.all_aff_shares_at_end_of_period = GreaterEqualZeroDecimal::try_from(
                        *sli.all_aff_shares_at_end_of_period - *after_tx_sell_shares
                    ).map_err(|_| {
                        // The caller may not have gone through those transactions to validate them
                        // yet, so we shouldn't panic here.
                        "Total share count went below zero in 30-day period after sale".to_string()
                    })?;
            },
            _ => (), // ignored
        }
    }

    // Finalize
    let did_buy_after_in_period = did_buy_after_in_period;

    if sli.all_aff_shares_at_end_of_period.is_zero() {
        // Not superficial
        return Ok(sli);
    }

    let mut did_buy_before_in_period = false;
    // Start just before the sell tx and work backwards
    for i in (0..=(idx - 1)).rev() {
        let before_tx = txs.get(i).unwrap();
        if before_tx.settlement_date < first_bad_buy_date {
            break;
        }
        let before_tx_affil = &before_tx.affiliate;
        // Within the 30 day window before
        match &before_tx.action_specifics {
            TxActionSpecifics::Buy(buy) => {
                did_buy_before_in_period = true;
                sli.total_aquired_in_period += GreaterEqualZeroDecimal::from(buy.shares);
                sli.buying_affiliates.insert(before_tx_affil.clone());
            },
            _ => (), // ignored
        }
    }

    sli.is_superficial = did_buy_before_in_period || did_buy_after_in_period;
    Ok(sli)
}

struct SflRatioResultResult {
    sfl_ratio: PosDecimalRatio, // TODO can this ever be zero?
    acb_adjust_affiliate_ratios: HashMap<Affiliate, PosDecimalRatio>,
    // ** Notes/warnings to emit later. **
    // Set when the sum of remaining involved affiliate shares is fewer than
    // the SFL shares, which means that the selling affiliate probably had some
    // shares they didn't sell. This can happen because we use interpretation/algo I.1
    // rather than I.2 (see the sfl wiki page) to determine the loss ratio.
    fewer_remaining_shares_than_sfl_shares: bool
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
fn get_superficial_loss_ratio(
    idx: usize, txs: Vec<Tx>, ptf_statuses: &AffiliatePortfolioSecurityStatuses)
    -> SflRatioResultResult {

    todo!()
}