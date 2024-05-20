use std::rc::Rc;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::{
    portfolio::{
        Affiliate, CurrencyAndExchangeRate, DeltaSflInfo,
        PortfolioSecurityStatus, SflaTxSpecifics,
        Tx, TxActionSpecifics, TxDelta
    },
    util::decimal::{
        GreaterEqualZeroDecimal, LessEqualZeroDecimal,
        NegDecimal, PosDecimal
    }
};

use super::{superficial_loss::get_superficial_loss_ratio, AffiliatePortfolioSecurityStatuses};

type Error = String;

// These are presumably possible to encounter from bad user input.
// Some of them at least.
fn sanity_check_ptfs(pre_tx_status: &Rc<PortfolioSecurityStatus>, tx: &Tx) -> Result<(), Error> {
    let err_prefix = || {
        format!("In {} transaction of {} on {}",
                tx.action(), tx.security, tx.trade_date.to_string())
    };

    if *pre_tx_status.all_affiliate_share_balance < *pre_tx_status.share_balance {
        Err(format!("{} the share balance across all affiliates \
        ({}) is lower than the share balance for the affiliate of the transaction ({})",
        err_prefix(), pre_tx_status.all_affiliate_share_balance, pre_tx_status.share_balance))
    } else if tx.affiliate.registered() && !pre_tx_status.total_acb.is_none() {
		Err(format!("{} found an ACB on a registered affiliate", err_prefix()))
	} else if !tx.affiliate.registered() && pre_tx_status.total_acb.is_none() {
		Err(format!("{} found an invalid ACB (none)", err_prefix()))
	} else {
        Ok(())
    }
}

// Caller must subtract the sfl from capital gains.
fn get_delta_superficial_loss_info(
        idx: usize, txs: &Vec<Tx>, ptf_statuses: &AffiliatePortfolioSecurityStatuses,
        cap_loss: NegDecimal,
    ) -> Result<Option<(DeltaSflInfo, Vec<Tx>)>, Error> {
    let tx = &txs[idx];

    let m_sfl = get_superficial_loss_ratio(idx, txs, ptf_statuses)?;

    let calculated_sfl_amount: LessEqualZeroDecimal = match &m_sfl {
        Some(sfl) => LessEqualZeroDecimal::from(cap_loss.mul_pos(sfl.sfl_ratio.to_posdecimal())),
        None => LessEqualZeroDecimal::zero(),
    };

    if let Some(specified_sfl) = &tx.sell_specifics().unwrap().specified_superficial_loss {
        if !specified_sfl.force {
            // Perform validation of specified loss against inferred value.
            let sfl_diff = (*calculated_sfl_amount - *specified_sfl.superficial_loss).abs();
            const MAX_DIFF: Decimal = rust_decimal_macros::dec!(0.001);
            if sfl_diff > MAX_DIFF {
                return Err(format!(
                        "Sell order on {} of {}: superficial loss was specified, but \
                        the difference between the specified value ({}) and the \
                        computed value ({}) is greater than the max allowed \
                        discrepancy ({}).\nTo force this SFL value, append an '!' \
                        to the value",
                    tx.trade_date, tx.security, specified_sfl.superficial_loss,
                    calculated_sfl_amount, MAX_DIFF));
            }
        }

        // Beyond this point, the specified loss is "sane" within reason, or was forced.

        let specified_loss = match NegDecimal::try_from(*specified_sfl.superficial_loss) {
            Ok(v) => v,
            Err(_) => {
                // Specified no loss
                return Ok(None);
            },
        };

        // We can't really rely on the computed ratio here, since it could be off
        // by a small amount (due to the diff allowance) or completely incorrect
        // if we forced. Produce a sensible-ish ratio (this is just for display purposes).
        // This could end up being greater than 1 in strange cases.

        let override_ratio = crate::util::math::PosDecimalRatio {
            numerator: (specified_loss / cap_loss) * tx.sell_specifics().unwrap().shares,
            denominator: tx.sell_specifics().unwrap().shares,
        };

        // Leave adjust_txs empty, since specified SFL must be accompanied with
        // Sfla Txs as input.
        let adjust_txs = Vec::new();
        Ok(Some((DeltaSflInfo {
            superficial_loss: specified_loss,
            ratio: override_ratio,
            potentially_over_applied: false,
        }, adjust_txs)))
    } else if let Some(sfl) = m_sfl {
        // Automatic SFL only

        // We don't need calculated_sfl_amount to be a LessEqualZeroDecimal anymore
        let calculated_sfl_amount = NegDecimal::try_from(*calculated_sfl_amount).unwrap();
        let potentially_over_applied_sfl = sfl.fewer_remaining_shares_than_sfl_shares;

        let mut acb_adjust_affiliates: Vec<&Affiliate> = sfl.acb_adjust_affiliate_ratios.keys().collect();
        acb_adjust_affiliates.sort_by(|a, b| { a.id().cmp(b.id()) });

        let mut adjust_txs = Vec::new();
        for af in acb_adjust_affiliates {
            let ratio_of_sfl = &sfl.acb_adjust_affiliate_ratios[af];
            if !ratio_of_sfl.numerator.is_zero() && !af.registered() {
                let af_ratio_posdecimal = PosDecimal::try_from(*ratio_of_sfl.to_gezdecimal()).unwrap();

                adjust_txs.push(Tx {
                    security: tx.security.clone(),
                    trade_date: tx.trade_date,
                    settlement_date: tx.settlement_date,
                    action_specifics: TxActionSpecifics::Sfla(SflaTxSpecifics {
                        // TODO this should be af_ratio_posdecimal.numerator, and amount_per_share should be
                        // just divided by the denominator instead of mult with the decimal.
                        shares_affected: PosDecimal::one(),
                        amount_per_share: NegDecimal::neg_1() * calculated_sfl_amount * af_ratio_posdecimal,
                    }),
                    memo: format!(
                        "Automatic SfL ACB adjustment. {:.2}% ({}) of SfL, which was {} of sale shares.",
                        (&(ratio_of_sfl.to_decimal() * dec!(100))),
                        ratio_of_sfl.to_string(), sfl.sfl_ratio,
                    ),
                    affiliate: af.clone(),
                    read_index: tx.read_index,
                });
            }
        }

        Ok(Some((DeltaSflInfo {
            superficial_loss: calculated_sfl_amount,
            ratio: sfl.sfl_ratio,
            potentially_over_applied: potentially_over_applied_sfl,
        }, adjust_txs)))
    } else {
        // Automatic, no SFL detected
        Ok(None)
    }
}

// Returns a TxDelta  for the Tx ad txs[idx]
// Optionally, returns a new Tx if a SFLA Tx was generated to accompany
// this Tx. It is expected that that Tx be inserted into txs and evaluated next.
fn delta_for_tx(idx: usize, txs: &Vec<Tx>, ptf_statuses: &AffiliatePortfolioSecurityStatuses)
    -> Result<(TxDelta, Option<Vec<Tx>>), Error> {

    let tx = &txs[idx];
    let pre_tx_status = ptf_statuses.get_next_pre_status(&tx.affiliate);

    assert_eq!(tx.security, pre_tx_status.security,
		"delta_for_tx: securities do not match ({} and {})",
        tx.security, pre_tx_status.security);

    // let total_local_share_price = *tx.shares * tx

    let total_local_share_value = |shares: PosDecimal,
                                   amount_per_share: GreaterEqualZeroDecimal,
                                   fx_rate: &CurrencyAndExchangeRate| -> GreaterEqualZeroDecimal {
        amount_per_share * shares.into() * fx_rate.exchange_rate.into()
    };

    let mut new_share_balance = pre_tx_status.share_balance;
    let mut new_all_affiliates_share_balance = pre_tx_status.all_affiliate_share_balance;
	let mut new_acb_total = pre_tx_status.total_acb;

    let mut capital_gains = if tx.affiliate.registered() { None } else { Some(Decimal::ZERO) };
    let mut sfl_info: Option<DeltaSflInfo> = None;
    let mut txs_to_inject: Option<Vec<Tx>> = None;

    sanity_check_ptfs(&pre_tx_status, tx)?;

    match &tx.action_specifics {
        crate::portfolio::TxActionSpecifics::Buy(buy_specs) => {
            new_share_balance = pre_tx_status.share_balance + buy_specs.shares.into();
            new_all_affiliates_share_balance = pre_tx_status.all_affiliate_share_balance +
                buy_specs.shares.into();
            if let Some(old_acb) = pre_tx_status.total_acb {
                let total_price = total_local_share_value(
                        buy_specs.shares, buy_specs.amount_per_share, &buy_specs.tx_currency_and_rate)
                    + (buy_specs.commission * buy_specs.commission_currency_and_rate().exchange_rate.into());
                new_acb_total = Some(old_acb + total_price);
            }
        },
        crate::portfolio::TxActionSpecifics::Sell(sell_specs) => {
            new_share_balance = GreaterEqualZeroDecimal::try_from(
                *pre_tx_status.share_balance - *sell_specs.shares)
                .map_err(|_| format!("Sell order on {} of {} shares of {} is more than the current holdings ({})",
                    tx.trade_date, sell_specs.shares, tx.security, pre_tx_status.share_balance))?;
            // TODO sanity check for this can probably be removed above.
            new_all_affiliates_share_balance = GreaterEqualZeroDecimal::try_from(
                *pre_tx_status.all_affiliate_share_balance - *sell_specs.shares)
                .map_err(|_| format!("Sell order on {} of {} shares of {} is more than the current \
                                      total holdings across affiliates ({})",
                    tx.trade_date, sell_specs.shares, tx.security, pre_tx_status.all_affiliate_share_balance))?;
		    // NOTE: commission plays no effect on sell order ACB
            if let Some(acb_per_share) = pre_tx_status.per_share_acb() {
                new_acb_total = Some(new_share_balance * acb_per_share);

                // We have an ACB, so we need to do capital gains calculations
                let total_payout = *total_local_share_value(
                    sell_specs.shares, sell_specs.amount_per_share, &sell_specs.tx_currency_and_rate)
                - *(sell_specs.commission * sell_specs.commission_currency_and_rate().exchange_rate.into());

                let unadjusted_cap_gains = total_payout - (*acb_per_share * *sell_specs.shares);
                capital_gains = Some(unadjusted_cap_gains);
                tracing::debug!(
                    "new_acb_total = {:#?}, total_payout = {:#?}, unadjusted_cap_gains = {:#?}",
                    new_acb_total, total_payout, unadjusted_cap_gains);

                if let Ok(cap_loss) = NegDecimal::try_from(unadjusted_cap_gains) {

                    let maybe_delta_sfl_info = get_delta_superficial_loss_info(
                        idx, txs, ptf_statuses, cap_loss)?;
                    if let Some((delta_sfl_info, adjust_txs)) = maybe_delta_sfl_info {
                        txs_to_inject = Some(adjust_txs);
                        capital_gains = Some(*cap_loss - *delta_sfl_info.superficial_loss);
                        sfl_info = Some(delta_sfl_info);
                    }

                } else if sell_specs.specified_superficial_loss.is_some() {
                    return Err(format!(
                        "Sell order on {} of {}: superficial loss was specified, but there is no capital loss",
                        tx.trade_date, tx.security));
                }
            }
        },
        crate::portfolio::TxActionSpecifics::Roc(roc_specs) => {
            if let Some(old_acb) = pre_tx_status.total_acb {
                assert!(!tx.affiliate.registered());
                let acb_reduction =
                    roc_specs.amount_per_held_share * pre_tx_status.share_balance *
                    roc_specs.tx_currency_and_rate.exchange_rate.into();
                new_acb_total = Some(GreaterEqualZeroDecimal::try_from(*old_acb - *acb_reduction)
                    .map_err(|_| format!("Invalid RoC tx on {}: RoC ({}) exceeds the current ACB ({})",
                        tx.trade_date, acb_reduction, old_acb))?);
            } else {
                assert!(tx.affiliate.registered());
                return Err(format!("Invalid RoC tx on {}: Registered affiliates do not have an ACB to adjust",
				    tx.trade_date));
            }
        },
        crate::portfolio::TxActionSpecifics::Sfla(sfla_specs) => {
            if let Some(old_acb) = pre_tx_status.total_acb {
                assert!(!tx.affiliate.registered());
                let acb_adjustment = sfla_specs.total_amount();
                new_acb_total = Some(old_acb + acb_adjustment.into());
            } else {
                assert!(tx.affiliate.registered());
                return Err(format!("Invalid SfLA tx on {}: Registered affiliates do not have an ACB to adjust",
				    tx.trade_date));
            }
        },
    }

    let new_status = PortfolioSecurityStatus {
        security: pre_tx_status.security.clone(),
        share_balance: new_share_balance,
        all_affiliate_share_balance: new_all_affiliates_share_balance,
        total_acb: new_acb_total,
    };

    let delta = TxDelta {
        tx: tx.clone(),
        pre_status: pre_tx_status.clone(),
        post_status: Rc::new(new_status),
        capital_gain: capital_gains,
        sfl: sfl_info,
    };

    Ok((delta, txs_to_inject))
}

pub fn txs_to_delta_list() {
    // TODO this is a placeholder for now

    // Reference our protected fns so they aren't
    // marked with warnings for unuse.
    let _ = delta_for_tx(
        0, &Vec::new(),
        &AffiliatePortfolioSecurityStatuses::new("foo".to_string(), None)
    );

    let r = get_superficial_loss_ratio(
        0, &Vec::new(),
        &AffiliatePortfolioSecurityStatuses::new("foo".to_string(), None)
    ).unwrap().unwrap();

    println!("{:#?}, {:#?}, {:#?}",
        r.sfl_ratio, r.acb_adjust_affiliate_ratios, r.fewer_remaining_shares_than_sfl_shares);
}