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

    let total_local_share_value = |shares: PosDecimal,
                                   amount_per_share: GreaterEqualZeroDecimal,
                                   fx_rate: &CurrencyAndExchangeRate| -> GreaterEqualZeroDecimal {
        amount_per_share * shares.into() * fx_rate.exchange_rate.into()
    };

    let mut new_share_balance = pre_tx_status.share_balance;
    let mut new_all_affiliates_share_balance = pre_tx_status.all_affiliate_share_balance;
    let mut new_acb_total = pre_tx_status.total_acb;

    let mut capital_gains: Option<Decimal> = None;
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

pub fn txs_to_delta_list(
        txs: &Vec<Tx>, initial_status: Option<Rc<PortfolioSecurityStatus>>,
    ) -> Result<Vec<TxDelta>, Error> {

    let mut active_txs: &Vec<Tx> = txs;
    // These will be populated if we end up injecting new Txs,
    // and active_txs will refer to them.
    let mut modified_txs: Option<Vec<Tx>> = None;

    let mut deltas = Vec::<TxDelta>::with_capacity(txs.len());

    if txs.len() == 0 {
        return Ok(deltas);
    }

    let mut ptf_statuses = AffiliatePortfolioSecurityStatuses::new(
        txs[0].security.clone(), initial_status);

    // Use a while loop here, since active_txs can grow while we iterate
    // it. DO NOT use `continue` in this loop!
    let mut i = 0;
    while i < active_txs.len() {
        let tx_affilliate = &active_txs[i].affiliate;
        let (delta, m_new_txs) = delta_for_tx(i, &active_txs, &ptf_statuses)?;

        tracing::trace!("txs_to_delta_list: adding post_status for delta: {:#?}", delta);
        ptf_statuses.set_latest_post_status(tx_affilliate, delta.post_status.clone());
        deltas.push(delta);

        if let Some(new_txs) = m_new_txs {
            if new_txs.len() > 0 {
                if modified_txs.is_none() {
                    modified_txs = Some(txs.clone());
                }
                let some_modified_txs = modified_txs.as_mut().unwrap();
                // Reserve _additional_ space.
                some_modified_txs.reserve(new_txs.len());
			    // Insert into modified_txs after the current Tx
                for (new_tx_i, new_tx) in new_txs.into_iter().enumerate() {
                    some_modified_txs.insert(i + new_tx_i + 1, new_tx);
                }
                active_txs = some_modified_txs;
            }
        }

        i += 1;
    }

    Ok(deltas)
}

// MARK: tests

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use crate::portfolio::bookkeeping::AffiliatePortfolioSecurityStatuses;
    use crate::portfolio::testlib::MAGIC_DEFAULT_CURRENCY;
    use crate::portfolio::Affiliate;
    use crate::portfolio::Currency;
    use crate::portfolio::PortfolioSecurityStatus;
    use crate::portfolio::SFLInput;
    use crate::portfolio::Tx;
    use crate::portfolio::TxAction as A;
    use crate::portfolio::TxActionSpecifics;
    use crate::portfolio::TxDelta;
    use crate::testlib::assert_big_struct_eq;
    use crate::testlib::assert_re;
    use crate::util::decimal::GreaterEqualZeroDecimal;
    use crate::util::decimal::LessEqualZeroDecimal;
    use crate::util::decimal::NegDecimal;
    use std::rc::Rc;

    use crate::portfolio::testlib::{default_sec, TTx};
    use crate::portfolio::bookkeeping::testlib::TPSS;

    use crate::gezdec as gez;
    use crate::lezdec as lez;

    fn usd() -> Currency {
        Currency::usd()
    }

    fn cad() -> Currency {
        Currency::cad()
    }

    macro_rules! sdec {
        ($arg:literal) => {{
            use rust_decimal_macros::dec;
            Some(dec!($arg))
        }};
    }

    macro_rules! sgez {
        ($arg:literal) => {{
            use crate::gezdec;
            Some(gezdec!($arg))
        }};
    }

    macro_rules! sndec {
        ($arg:literal) => {{
            use crate::ndec;
            Some(ndec!($arg))
        }};
    }

    // Shortening alias
    fn def<T: Default>() -> T {
        Default::default()
    }

    fn delta_for_tx(tx: Tx, pre_tx_status: Rc<PortfolioSecurityStatus>)
        -> Result<(TxDelta, Option<Vec<Tx>>), super::Error> {
        let mut ptf_statuses = AffiliatePortfolioSecurityStatuses::new(tx.security.clone(), None);
        let share_diff = GreaterEqualZeroDecimal::try_from(
            *pre_tx_status.all_affiliate_share_balance - *pre_tx_status.share_balance).unwrap();
        // Set up the previous balance to avoid assert
        let dummy_af = Affiliate::from_strep("dummy(R)");
        ptf_statuses.set_latest_post_status(&dummy_af, TPSS{shares: share_diff, ..def()}.x());
        ptf_statuses.set_latest_post_status(&tx.affiliate, pre_tx_status);
        let txs = vec![tx];

        super::delta_for_tx(0, &txs, &ptf_statuses)
    }

    fn delta_for_tx_ok(tx: Tx, sptf: &PortfolioSecurityStatus) -> TxDelta {
        delta_for_tx(tx, Rc::new(sptf.clone())).unwrap().0
    }

    fn delta_for_tx_has_err(tx: Tx, sptf: &PortfolioSecurityStatus) {
        delta_for_tx(tx, Rc::new(sptf.clone())).unwrap_err();
    }

    fn delta_for_tx_get_err(tx: Tx, sptf: &PortfolioSecurityStatus) -> super::Error {
        delta_for_tx(tx, Rc::new(sptf.clone())).unwrap_err()
    }

    fn validate_delta(delta: TxDelta, tdt: TDt) {
        assert_big_struct_eq(delta.post_status, tdt.post_st.x());
        assert_eq!(delta.capital_gain, tdt.gain);
        assert_eq!(delta.sfl.map(|s| s.superficial_loss), tdt.sfl);
    }

    fn txs_to_delta_list_no_err(txs: Vec<Tx>) -> Vec<TxDelta> {
        super::txs_to_delta_list(&txs, None).unwrap()
    }

    fn txs_to_delta_list_with_err(txs: Vec<Tx>) {
        super::txs_to_delta_list(&txs, None).unwrap_err();
    }

    fn validate_deltas(deltas: Vec<TxDelta>, exp: Vec<TDt>) {
        // assert_vec_eq(left, right)
        if deltas.len() != exp.len() {
            crate::testlib::eprint_big_struct_vec(&deltas);
            assert_eq!(deltas.len(), exp.len(), "Lengths must be equal");
        }

        for (i, delta) in deltas.iter().enumerate() {
            let tdt = &exp[i];
            let post_st = tdt.post_st.x();
            if delta.post_status != post_st ||
                delta.capital_gain != tdt.gain ||
                delta.sfl.as_ref().map(|s| &s.superficial_loss) != tdt.sfl.as_ref() ||
                delta.sfl.as_ref().map(|s| s.potentially_over_applied).unwrap_or(false) != tdt.potentially_over_applied_sfl
            {
                crate::testlib::eprint_big_struct_vec(&deltas);
                // assert!(false, "At index {}: {:#?} ~!= {:#?}", i, delta, tdt);
                assert!(false, "At index {}: \n\
                    actual:\n  \
                      post_st: {:#?}\n  \
                      capital_gain: {:#?}\n  \
                      sfl: {:#?}\n  \
                    \n\
                    expected:\n  \
                        post_st: {:#?}\n  \
                        capital_gain: {:#?}\n  \
                        sfl: {:#?}\n  \
                        potentially_over_applied_sfl: {}",
                    i,
                    delta.post_status, delta.capital_gain, delta.sfl,
                    post_st, tdt.gain, tdt.sfl, tdt.potentially_over_applied_sfl,
                );
            }
        }
    }

    fn cadsfl(superficial_loss: LessEqualZeroDecimal, force: bool) -> Option<SFLInput> {
        Some(crate::portfolio::SFLInput{
            superficial_loss: superficial_loss,
            force: force,
        })
    }

    // Test Delta
    #[derive(Debug)]
    struct TDt {
        pub post_st:                    TPSS,
        pub gain:                      Option<Decimal>,
        pub sfl:                       Option<NegDecimal>,
        pub potentially_over_applied_sfl: bool,
    }

    impl Default for TDt {
        fn default() -> Self {
            Self {
                post_st: Default::default(),
                gain: Default::default(),
                sfl: Default::default(),
                potentially_over_applied_sfl: Default::default()
            }
        }
    }

    #[test]
    fn test_basic_buy_acb() {

        let sptf = Rc::new(PortfolioSecurityStatus {
            security: default_sec(),
            share_balance: gez!(0),
            all_affiliate_share_balance: gez!(0),
            total_acb: Some(gez!(0)),
        });

        // Basic Buy
        let tx = TTx{act: A::Buy, shares: gez!(3), price: gez!(10.0), ..def()}.x();
        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(3), total_acb: sgez!(30.0), ..def()}, ..def()});

        // Test with commission
        let tx = TTx{act: A::Buy, shares: gez!(2), price: gez!(10.0), comm: gez!(1.0), ..def()}.x();
        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(2), total_acb: sgez!(21.0), ..def()}, ..def()});

        // Test with exchange rates
        let sptf = TPSS{shares: gez!(2), total_acb: sgez!(21.0), ..def()}.x();
        let tx = TTx{act: A::Buy, shares: gez!(3), price: gez!(12.0), comm: gez!(1.0),
            curr: usd(), fx_rate: gez!(2.0),
            comm_curr: Currency::new("XXX"), comm_fx_rate: gez!(0.3), ..def()}.x();
        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{
                    shares: gez!(5),
                    // 21 + (12 * 2 * 3 = 72) + 0.3
                    total_acb: Some(gez!(21.0) + gez!(72.0) + gez!(0.3)), ..def()},
                ..def()});
    }

    #[test]
    fn test_basic_sell_acb_errors() {
        // Sell more shares than available
        let sptf = TPSS{shares: gez!(2), total_acb: sgez!(20.0), ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(3), price: gez!(10.0), ..def()}.x();
        delta_for_tx_has_err(tx, &sptf);
    }

    #[test]
    fn test_basic_sell_acb() {

        // Sell all remaining shares
        let sptf = TPSS{shares: gez!(2), total_acb: sgez!(20.0), ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(15.0), ..def()}.x();

        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0), ..def()}, gain: sdec!(10.0), ..def()});

        // Sell shares with commission
        let sptf = TPSS{shares: gez!(3), total_acb: sgez!(30.0), ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(15.0), comm: gez!(1.0), ..def()}.x();

        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(1), total_acb: sgez!(10.0), ..def()}, gain: sdec!(9.0), ..def()});

        // Sell shares with exchange rate
        let sptf = TPSS{shares: gez!(3), total_acb: sgez!(30.0), ..def()}.x();
        let tx = TTx{
            act: A::Sell, shares: gez!(2), price: gez!(15.0), comm: gez!(2.0),
            curr: Currency::new("XXX"), fx_rate: gez!(2.0),
            comm_curr: Currency::new("YYY"), comm_fx_rate: gez!(0.4), ..def()}.x();

        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            // ((15.0 * 2.0 * 2.0) - 20.0 - 0.8) = 39.2
            TDt{post_st: TPSS{shares: gez!(1), total_acb: sgez!(10.0), ..def()}, gain: sdec!(39.2), ..def()});
    }

    #[test]
    fn test_superficial_losses() {

        /*
            buy 10
            wait
            sell 5 (loss, not superficial)
        */
        let mut txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell half at a loss a while later, for a total of $1
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(12.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(6.0), ..def()}, gain: sdec!(-5.0), ..def()},
        ]);

        // (min(#sold, totalAquired, endBalance) / #sold) x (Total Loss)

        /*
            buy 10
            sell 5 (superficial loss) -- min(5, 10, 1) / 5 * (loss of $5) = 1
            sell 4 (superficial loss) -- min(4, 10, 1) / 4 * (loss of $4.8) = 0.6
            wait
            sell 1 (loss, not superficial)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell soon, causing superficial losses
            TTx{t_day: 2, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(),
            TTx{t_day: 15, act: A::Sell, shares: gez!(4), price: gez!(0.2), ..def()}.x(),
            // Normal sell a while later
            TTx{t_day: 100, act: A::Sell, shares: gez!(1), price: gez!(0.2), ..def()}.x(),
        ];

        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(12.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(6.0), ..def()}, gain: sdec!(-4.0), sfl: sndec!(-1), ..def()},      // $1 superficial
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.0), ..def()}, ..def()}, // acb adjust
            TDt{post_st: TPSS{shares: gez!(1), total_acb: sgez!(1.4), ..def()}, gain: sdec!(-3.6), sfl: sndec!(-1.2), ..def()},      // $1.2 superficial
            TDt{post_st: TPSS{shares: gez!(1), total_acb: sgez!(2.6), ..def()}, ..def()}, // acb adjust
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0), ..def()}, gain: sdec!(-2.4), ..def()},
        ]);

        /*
            buy 10
            wait
            sell 5 - loss of $5 (superficial loss) -- min(5, 5, 10) / 5 = 1 (100%)
            buy 5
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(),
            TTx{t_day: 51, act: A::Buy, shares: gez!(5), price: gez!(0.2), comm: gez!(2.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(12.0), ..def()}, ..def()}, // buy
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(6.0), ..def()}, gain: sdec!(0), sfl: sndec!(-5), ..def()},   // sell sfl $5
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(11.0), ..def()}, ..def()},  // sfl ACB adjust
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(14.0), ..def()}, ..def()}, // buy
        ]);

        /*
            USD SFL test.
            buy 10 (in USD)
            wait
            sell 5 (in USD) (loss of 6.0 cad) (superficial loss) -- min(5, 5, 10) / 5 = 1 (100%)
            buy 5 (in USD)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
            // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), ..def()}.x(),
            TTx{t_day: 51, act: A::Buy, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(14.4), ..def()}, ..def()}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.2), ..def()}, gain: sdec!(0), sfl: sndec!(-6.0), ..def()},   // sell sfl $5 USD (6 CAD)
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(13.2), ..def()}, ..def()},  // sfl ACB adjust
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(16.8), ..def()}, ..def()}, // buy
        ]);

        /*
            buy 10
            wait
            sell 5 (loss)
            sell 5 (loss)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(),
            TTx{t_day: 51, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(12.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(6.0), ..def()}, gain: sdec!(-5.0), ..def()},
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0), ..def()}, gain: sdec!(-5.0), ..def()},
        ]);

        /*
            buy 100
            wait
            sell 99 (loss of −104) (superficial loss) -- min(99, 25, 26) / 99 = 0.252525253
            buy 25
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(100), price: gez!(3.0), comm: gez!(2.0), ..def()}.x(), // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(99), price: gez!(2.0), ..def()}.x(),
            TTx{t_day: 51, act: A::Buy, shares: gez!(25), price: gez!(2.2), comm: gez!(2.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(100), total_acb: sgez!(302.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(1), total_acb: sgez!(3.02), ..def()},
                    gain: sdec!(-75.480000000000000000000000003), sfl: sndec!(-25.499999999999999999999999997), ..def()},  // total loss of 100.98, 25.500000048 is superficial
            TDt{post_st: TPSS{shares: gez!(1), total_acb: sgez!(28.519999999999999999999999997), ..def()}, ..def()}, // acb adjust
            TDt{post_st: TPSS{shares: gez!(26), total_acb: sgez!(85.52000000000000000000000000), ..def()}, ..def()},
        ]);

        /*
            buy 10
            sell 10 (superficial loss) -- min(10, 15, 3) / 10 = 0.3
            buy 5
            sell 2 (superficial loss) -- min(2, 15, 3) / 2 = 1
            wait
            sell 3 (loss)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell all
            TTx{t_day: 2, act: A::Sell, shares: gez!(10), price: gez!(0.2), ..def()}.x(),
            TTx{t_day: 3, act: A::Buy, shares: gez!(5), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            TTx{t_day: 4, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),
            TTx{t_day: 50, act: A::Sell, shares: gez!(3), price: gez!(0.2), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(12.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0), ..def()}, gain: sdec!(-7), sfl: sndec!(-3), ..def()},        // Superficial loss of 3
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(3), ..def()}, ..def()}, // acb adjust
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(10.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(3), total_acb: sgez!(6.0), ..def()}, gain: sdec!(0), sfl: sndec!(-3.6), ..def()}, // Superficial loss of 3.6
            TDt{post_st: TPSS{shares: gez!(3), total_acb: sgez!(9.6), ..def()}, ..def()}, // acb adjust
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0), ..def()}, gain: sdec!(-9), ..def()},
        ]);

        /*
            buy 10
            sell 5 (gain)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell causing gain
            TTx{t_day: 2, act: A::Sell, shares: gez!(5), price: gez!(2.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(12.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(6.0), ..def()}, gain: sdec!(4.0), ..def()},
        ]);

        /* Fractional shares SFL avoidance
           With floats, this would be hard, because we wouldn't come exactly back to zero.
           We get around this by using Decimal

           buy 5.0
           sell 4.7
           sell 0.3 (loss) (not superficial because we sold all shares and should have zero)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(5.0), price: gez!(1.0), comm: gez!(2.0), ..def()}.x(),
            // Sell all in two fractional operations
            TTx{t_day: 2, act: A::Sell, shares: gez!(4.7), price: gez!(0.2), ..def()}.x(),
            TTx{t_day: 3, act: A::Sell, shares: gez!(0.3), price: gez!(0.2), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(0.3), total_acb: sgez!(0.42), ..def()}, gain: sdec!(-5.64), ..def()},
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0), ..def()}, gain: sdec!(-0.36), ..def()},
        ]);

        // ************** Explicit Superficial Losses ***************************
        // Accurately specify a detected SFL
        /*
            USD SFL test.
            buy 10 (in USD)
            wait
            sell 5 (in USD) (loss of ) (superficial loss) min(5, 10, 5) / 5 = 100%
            buy 5 (in USD)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
            // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), sfl: cadsfl(lez!(-6.0), false), ..def()}.x(),
            // ACB adjust is partial, as if splitting some to another affiliate.
            TTx{t_day: 50, act: A::Sfla, shares: gez!(5), price: gez!(0.02), curr: MAGIC_DEFAULT_CURRENCY.clone(), fx_rate: gez!(1.0), ..def()}.x(),
            TTx{t_day: 51, act: A::Buy, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(14.4), ..def()}, ..def()}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.2), ..def()}, gain: sdec!(0), sfl: sndec!(-6), ..def()},   // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 6 CAD
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.3), ..def()}, ..def()},   // sfl ACB adjust 0.02 * 5
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(10.9), ..def()}, ..def()}, // buy
        ]);

        // Override a detected SFL
        /*
            USD SFL test.
            buy 10 (in USD)
            wait
            sell 5 (in USD) (superficial loss)
            buy 5 (in USD)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
            // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), sfl: cadsfl(lez!(-0.7), true), ..def()}.x(),
            // ACB adjust is partial, as if splitting some to another affiliate.
            TTx{t_day: 50, act: A::Sfla, shares: gez!(5), price: gez!(0.02), curr: MAGIC_DEFAULT_CURRENCY.clone(), fx_rate: gez!(1.0), ..def()}.x(),
            TTx{t_day: 51, act: A::Buy, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs.clone());
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(14.4), ..def()}, ..def()}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.2), ..def()}, gain: sdec!(-5.3), sfl: sndec!(-0.7), ..def()},  // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.3), ..def()}, ..def()},   // sfl ACB adjust 0.02 * 5
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(10.9), ..def()}, ..def()}, // buy
        ]);

        // Un-force the override, and check that we emit an error
        // Expect an error since we did not force.
        let mut tx1_sell_specs = txs[1].sell_specifics().unwrap().clone();
        tx1_sell_specs.specified_superficial_loss = cadsfl(lez!(-0.7), false);
        txs.get_mut(1).unwrap().action_specifics = TxActionSpecifics::Sell(tx1_sell_specs);
        txs_to_delta_list_with_err(txs);

        // Add an un-detectable SFL (ie, the buy occurred in an untracked affiliate)
        /*
            USD SFL test.
            buy 10 (in USD)
            wait
            sell 5 (in USD) (loss)
        */
        txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), curr: usd(), fx_rate: gez!(1.2), comm: gez!(2.0), ..def()}.x(),
            // Sell causing superficial loss, because of quick buyback
            TTx{t_day: 50, act: A::Sell, shares: gez!(5), price: gez!(0.2), curr: usd(), fx_rate: gez!(1.2), sfl: cadsfl(lez!(-0.7), true), ..def()}.x(),
            // ACB adjust is partial, as if splitting some to another affiliate.
            TTx{t_day: 50, act: A::Sfla, shares: gez!(5), price: gez!(0.02), curr: cad(), fx_rate: gez!(1.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs.clone());
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), total_acb: sgez!(14.4), ..def()}, ..def()}, // buy, ACB (CAD) = (10*1.0 + 2) * 1.2
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.2), ..def()}, gain: sdec!(-5.3), sfl: sndec!(-0.7), ..def()}, // sell for $1 USD, capital loss $-5 USD before SFL deduction, sfl 0.7 CAD
            TDt{post_st: TPSS{shares: gez!(5), total_acb: sgez!(7.3), ..def()}, ..def()},   // sfl ACB adjust 0.02 * 5
        ]);

        // Un-force the override, and check that we emit an error
        // Expect an error since we did not force.
        let mut tx1_sell_specs = txs[1].sell_specifics().unwrap().clone();
        tx1_sell_specs.specified_superficial_loss = cadsfl(lez!(-0.7), false);
        txs.get_mut(1).unwrap().action_specifics = TxActionSpecifics::Sell(tx1_sell_specs);
        txs_to_delta_list_with_err(txs);

        // Currency errors
        // Sanity check for ok by itself.
        txs = vec![
            TTx{t_day: 50, act: A::Sfla, shares: gez!(1), price: gez!(0.1), curr: cad(), fx_rate: gez!(1.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0.1), ..def()}, ..def()},
        ]);

        txs = vec![
            TTx{t_day: 50, act: A::Sfla, shares: gez!(1), price: gez!(0.1), curr: MAGIC_DEFAULT_CURRENCY.clone(), fx_rate: gez!(1.0), ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(0), total_acb: sgez!(0.1), ..def()}, ..def()},
        ]);
    }

    #[test]
    fn test_basic_roc_acb_errors() {
        // Test that RoC cannot exceed the current ACB
        let sptf = TPSS{shares: gez!(2), total_acb: sgez!(20.0), ..def()}.x();
        let tx = TTx{act: A::Roc, price: gez!(13.0), ..def()}.x();
        delta_for_tx_has_err(tx, &sptf);

        // Test that RoC cannot occur on registered affiliates, since they have no ACB
        let sptf = TPSS{shares: gez!(5), total_acb: None, ..def()}.x();
        let tx = TTx{act: A::Roc, price: gez!(3.0), af_name: "(R)", ..def()}.x();
        delta_for_tx_has_err(tx, &sptf);
    }

    #[test]
    fn test_basic_roc_acb() {

        // Test basic ROC with different AllAffiliatesShareBalance
        let sptf = TPSS{shares: gez!(2), all_shares: gez!(8), total_acb: sgez!(20.0), ..def()}.x();
        let tx = TTx{act: A::Roc, price: gez!(1.0), ..def()}.x();

        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(2), all_shares: gez!(8), total_acb: sgez!(18.0), ..def()}, ..def()});

        // Test RoC with exchange
        let sptf = TPSS{shares: gez!(2), total_acb: sgez!(20.0), ..def()}.x();
        let tx = TTx{act: A::Roc, price: gez!(1.0), curr: usd(), fx_rate: gez!(2.0), ..def()}.x();

        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(2), total_acb: sgez!(16.0), ..def()}, ..def()});
    }

    #[test]
    fn test_basic_sfla_errors() {

        // Test than an SfLA on a registered affiliate is invalid
        let sptf = TPSS{shares: gez!(2), total_acb: None, ..def()}.x();
        let tx = TTx{act: A::Sfla, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x();
        let err = delta_for_tx_get_err(tx, &sptf);
        assert_re("Registered affiliates do not have an ACB", &err)
    }

    #[test]
    fn test_registered_affiliate_capital_gain() {
        // Test there are no capital gains in registered accounts
        let sptf = TPSS{shares: gez!(5), total_acb: None, ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(3.0), af_name: "(R)", ..def()}.x();
        let delta = delta_for_tx_ok(tx, &sptf);
        assert_big_struct_eq(TPSS{shares: gez!(3), acb_per_sh: None, ..def()}.x(), delta.post_status);
        assert_eq!(delta.capital_gain, None);
    }

    #[test]
    #[should_panic]
    fn test_registered_affiliate_acb_panic() {
        // Test that we fail if registered account sees non-nan acb
        let sptf = TPSS{shares: gez!(5), total_acb: sgez!(0), ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(3.0), af_name: "(R)", ..def()}.x();
        let _ = delta_for_tx(tx, sptf);
    }

    #[test]
    #[should_panic]
    fn test_registered_affiliate_acb_panic_nonzero() {
        // Test that we fail if registered account has non-zero acb
        let sptf = TPSS{shares: gez!(5), total_acb: sgez!(1.0), ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(3.0), af_name: "(R)", ..def()}.x();
        let _ = delta_for_tx(tx, sptf);
    }

    #[test]
    #[should_panic]
    fn test_non_registered_affiliate_no_acb_panic() {
        // Test that non-registered with None ACB generates an error as well
        let sptf = TPSS{shares: gez!(5), total_acb: None, ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(3.0), ..def()}.x();
        let _ = delta_for_tx(tx, sptf);
    }

    #[test]
    fn test_all_affiliate_share_balance_add_tx() {

        // Basic buy
        let sptf = TPSS{shares: gez!(3), all_shares: gez!(7), total_acb: sgez!(15.0), ..def()}.x();
        let tx = TTx{act: A::Buy, shares: gez!(2), price: gez!(5.0), ..def()}.x();
        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(9), total_acb: sgez!(25.0), ..def()}, ..def()});

        // Basic sell
        let sptf = TPSS{shares: gez!(5), all_shares: gez!(8), acb_per_sh: sgez!(3.0), ..def()}.x();
        let tx = TTx{act: A::Sell, shares: gez!(2), price: gez!(5.0), ..def()}.x();
        let delta = delta_for_tx_ok(tx, &sptf);
        validate_delta(delta,
            TDt{post_st: TPSS{shares: gez!(3), all_shares: gez!(6.0), acb_per_sh: sgez!(3.0), ..def()}, gain: sdec!(4.0), ..def()});
    }

    #[test]
    fn test_multi_affiliate_gains() {

        /*
            Default                Default (R)            B                    B (R)
            --------                ------------        ---------        ------------
            buy 10                  buy 20                buy 30            buy 40
            sell 1 (gain)
                                    sell 2 ("gain")
                                                            sell 3 (gain)
                                                                                sell 4 ("gain")
        */
        let txs = vec![
            // Buys
            TTx{act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{act: A::Buy, shares: gez!(20), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{act: A::Buy, shares: gez!(30), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{act: A::Buy, shares: gez!(40), price: gez!(1.0), af_name: "B (R)", ..def()}.x(),
            // Sells
            TTx{act: A::Sell, shares: gez!(1), price: gez!(1.2), af_name: "", ..def()}.x(),
            TTx{act: A::Sell, shares: gez!(2), price: gez!(1.3), af_name: "(R)", ..def()}.x(),
            TTx{act: A::Sell, shares: gez!(3), price: gez!(1.4), af_name: "B", ..def()}.x(), TTx{act: A::Sell, shares: gez!(4), price: gez!(1.5), af_name: "B (R)", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            // Buys
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), acb_per_sh: sgez!(1.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(20), all_shares: gez!(30), total_acb: None, ..def()}, gain: None, ..def()},
            TDt{post_st: TPSS{shares: gez!(30), all_shares: gez!(60), acb_per_sh: sgez!(1.0), ..def()}, ..def()},
            TDt{post_st: TPSS{shares: gez!(40), all_shares: gez!(100), total_acb: None, ..def()}, gain: None, ..def()},
            // Sells
            TDt{post_st: TPSS{shares: gez!(9), all_shares: gez!(99), acb_per_sh: sgez!(1.0), ..def()}, gain: sdec!(0.2), ..def()}, // 1 * 0.2
            TDt{post_st: TPSS{shares: gez!(18), all_shares: gez!(97), total_acb: None, ..def()}, gain: None, ..def()},
            TDt{post_st: TPSS{shares: gez!(27), all_shares: gez!(94), acb_per_sh: sgez!(1.0), ..def()}, gain: sdec!(1.2), ..def()}, // 3 * 0.4 = 1.2
            TDt{post_st: TPSS{shares: gez!(36), all_shares: gez!(90), total_acb: None, ..def()}, gain: None, ..def()},
        ]);
    }

    #[test]
    fn test_multi_affiliate_roc() {

        /*
            Default                B
            --------                ------------
            buy 10                  buy 20
                                    ROC
            sell 10                sell 20
        */
        let txs = vec![
            // Buys
            TTx{act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{act: A::Buy, shares: gez!(20), price: gez!(1.0), af_name: "B", ..def()}.x(),
            // ROC
            TTx{act: A::Roc, price: gez!(0.2), af_name: "B", ..def()}.x(),
            // Sells
            TTx{act: A::Sell, shares: gez!(10), price: gez!(1.1), af_name: "", ..def()}.x(),
            TTx{act: A::Sell, shares: gez!(20), price: gez!(1.1), af_name: "B", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            // Buys
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), acb_per_sh: sgez!(1.0), ..def()}, ..def()}, // Default
            TDt{post_st: TPSS{shares: gez!(20), all_shares: gez!(30), acb_per_sh: sgez!(1.0), ..def()}, ..def()}, // B
            // ROC
            TDt{post_st: TPSS{shares: gez!(20), all_shares: gez!(30), acb_per_sh: sgez!(0.8), ..def()}, ..def()}, // B
            // Sells
            TDt{post_st: TPSS{shares: gez!(0), all_shares: gez!(20), acb_per_sh: sgez!(0), ..def()}, gain: sdec!(1.0), ..def()}, // 10 * 0.1 = 1.0 : Default
            TDt{post_st: TPSS{shares: gez!(0), all_shares: gez!(0), acb_per_sh: sgez!(0), ..def()}, gain: sdec!(6.0), ..def()}, // 20 * 0.3 = 6.0 : B
        ]);
    }

    #[test]
    fn test_other_affiliate_sfl() {

        /* SFL with all buys on different affiliate

        Default                B
        --------                ------------
        buy 10                buy 5
        wait...
        sell 2 (SFL)
                                buy 2
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                 // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), total_acb: sgez!(5.0), ..def()}, ..def()},                   // Buy in B
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(13), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()}, // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(13), total_acb: sgez!(6.0), ..def()}, ..def()},                   // Auto-adjust on B
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(15), total_acb: sgez!(8.0), ..def()}, ..def()},                   // B
        ]);

        /* SFL with all buys on registered affiliate
           (same txs as above)
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                             // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), total_acb: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(13), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()},             // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(15), total_acb: None, ..def()}, gain: None, ..def()}, // Buy in (R)
        ]);

        /* SFL with all buys on other affiliate B, but sells on a second affiliate (R)
        Make sure it doesn't interfere or cause errors.
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 41, act: A::Sell, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                             // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), total_acb: sgez!(5.0), ..def()}, ..def()},                               // Buy in B
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(20), total_acb: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(18), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()}, // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(18), total_acb: sgez!(6.0), ..def()}, ..def()},                               // Auto-adjust on B
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(20), total_acb: sgez!(8.0), ..def()}, ..def()},                               // Buy in B
            TDt{post_st: TPSS{shares: gez!(3), all_shares: gez!(18), total_acb: None, ..def()}, gain: None, ..def()}, // Sell in (R)
        ]);

        /* SFL with buys on two other affiliates (both non-registered)
        Default            B            C
        --------            ------    -------
        buy 10            buy 5        buy 7
        wait...
        sell 2 (SFL)
                            buy 2        buy 2
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(7), price: gez!(1.0), af_name: "C", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "C", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                 // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), total_acb: sgez!(5.0), ..def()}, ..def()},                   // Buy in B
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: sgez!(7.0), ..def()}, ..def()},                   // Buy in C
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(20), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()}, // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(20), total_acb: sgez!(5.4375), ..def()}, ..def()},                // Auto-adjust on B. Gets 7/16 (43.75%) of the SFL
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(20), total_acb: sgez!(7.5625), ..def()}, ..def()},                // Auto-adjust on C. Gets 9/16 (56.25%) of the SFL
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: sgez!(7.4375), ..def()}, ..def()},                // Buy in B
            TDt{post_st: TPSS{shares: gez!(9), all_shares: gez!(24), total_acb: sgez!(9.5625), ..def()}, ..def()},                // Buy in C
        ]);

        /* SFL with buys on two other affiliates (registered/non-registered)
        Default            (R)        B
        --------            ------    -------
        buy 10            buy 5        buy 7
        wait...
        sell 2 (SFL)
                            buy 2        buy 2
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(7), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                             // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), acb_per_sh: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: sgez!(7.0), ..def()}, ..def()},                               // Buy in B
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(20), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()},             // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(20), total_acb: sgez!(7.5625), ..def()}, ..def()},                            // Auto-adjust on B. Gets 9/16 (56.25%) of the SFL
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(9), all_shares: gez!(24), total_acb: sgez!(9.5625), ..def()}, ..def()},                            // Buy in B
        ]);

        /* SFL with buys on one other affiliate, but fewer shares in the only selling
        affiliate than the shares affected by the superficial loss.

        Default            B
        --------            ------------
        buy 5
        wait...
        sell 4 (SFL)
                            buy 2
                            sell 1
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(4), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 42, act: A::Sell, shares: gez!(1), price: gez!(2.0), af_name: "B", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(5), total_acb: sgez!(5.0), ..def()}, ..def()}, // Buy in Default
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: sgez!(1.0), ..def()}, gain: sdec!(-1.0), sfl: sndec!(-1.0),
                potentially_over_applied_sfl: true}, // SFL of 0.5 * 2(/4) shares
            TDt{post_st: TPSS{shares: gez!(0), all_shares: gez!(1), total_acb: sgez!(1.0), ..def()}, ..def()},               // auto adjust on B (100%)
            TDt{post_st: TPSS{shares: gez!(2), all_shares: gez!(3), total_acb: sgez!(3.0), ..def()}, ..def()},                    // Buy in B
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(2), total_acb: sgez!(1.5), ..def()}, gain: sdec!(0.50), ..def()}, // Sell in B
        ]);

        /* SFL with buys on both SFL affiliate and one other affiliate.

        Default            B
        --------            ------------
        buy 5
        wait...
        sell 4 (SFL)
                            buy 2
        buy 1
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(4), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 42, act: A::Buy, shares: gez!(1), price: gez!(2.0), af_name: "", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(5), total_acb: sgez!(5.0), ..def()}, ..def()},                                      // Buy in Default
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: sgez!(1.0), ..def()}, gain: sdec!(-0.5), sfl: sndec!(-1.5), ..def()}, // SFL of 0.5 * 3(/4) shares
            TDt{post_st: TPSS{shares: gez!(0), all_shares: gez!(1), total_acb: sgez!(0.75), ..def()}, ..def()},                                // auto adjust on B (50%)
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: sgez!(1.75), ..def()}, ..def()},                                     // auto adjust on default (50%)
            TDt{post_st: TPSS{shares: gez!(2), all_shares: gez!(3), total_acb: sgez!(2.75), ..def()}, ..def()},                                     // Buy in B
            TDt{post_st: TPSS{shares: gez!(2), all_shares: gez!(4), total_acb: sgez!(3.75), ..def()}, ..def()},                                     // Buy in default
        ]);

        /* SFL with buy on one other registered affiliate.

        Default            (R)
        --------            ------------
        buy 5
        wait...
        sell 4 (SFL)
                            buy 2
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(4), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(5), total_acb: sgez!(5.0), ..def()}, ..def()},                                      // Buy in Default
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: sgez!(1.0), ..def()}, gain: sdec!(-1.0), sfl: sndec!(-1.0), ..def()}, // SFL of 0.5 * 2(/4) shares
            TDt{post_st: TPSS{shares: gez!(2), all_shares: gez!(3), total_acb: None, ..def()}, gain: None, ..def()},        // Buy in B
        ]);

        /* SFL with buy on one other registered affiliate, but fewer shares in the only
        selling affiliate than the shares affected by the superficial loss.

        Default            (R)
        --------            ------------
        buy 5
        wait...
        sell 4 (SFL)
                            buy 2
                            sell 1
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(4), price: gez!(0.5), af_name: "", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 42, act: A::Sell, shares: gez!(1), price: gez!(2.0), af_name: "(R)", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(5), total_acb: sgez!(5.0), ..def()}, ..def()}, // Buy in Default
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(1), total_acb: sgez!(1.0), ..def()}, gain: sdec!(-1.0), sfl: sndec!(-1.0),
                potentially_over_applied_sfl: true}, // SFL of 0.5 * 2(/4) shares
            TDt{post_st: TPSS{shares: gez!(2), all_shares: gez!(3), total_acb: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(1), all_shares: gez!(2), total_acb: None, ..def()}, gain: None, ..def()}, // Sell in (R)
        ]);
    }

    #[test]
    fn test_other_affiliate_explicit_sfl() {

        /* SFL with sells on two other affiliates (both non-registered),
           and explicitly set the SFLA (dubiously) on one of the affiliates.
        Default            B            C
        --------            ------    -------
        buy 10            buy 5        buy 7
        wait...
        sell 2 (explicit SFL)
                                SFLA
                            buy 2        buy 2
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(7), price: gez!(1.0), af_name: "C", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", sfl: cadsfl(lez!(-1.0), false), ..def()}.x(),
            TTx{t_day: 40, act: A::Sfla, shares: gez!(1), price: gez!(0.5), af_name: "C", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "C", ..def()}.x(),
        ];
        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                 // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), acb_per_sh: sgez!(1.0), ..def()}, ..def()},                   // Buy in B
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: sgez!(7.0), ..def()}, ..def()},                   // Buy in C
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(20), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()}, // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(20), total_acb: sgez!(7.5), ..def()}, ..def()},                   // Explicit adjust on C
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), acb_per_sh: sgez!(1.0), ..def()}, ..def()},                   // Buy in B
            TDt{post_st: TPSS{shares: gez!(9), all_shares: gez!(24), total_acb: sgez!(9.5), ..def()}, ..def()},                   // Buy in C
        ]);

        /* SFL with sells on two other affiliates (registered/non-registered),
            with explicit SFL
        Default            (R)        B
        --------            ------    -------
        buy 10            buy 5        buy 7
        wait...
        sell 2 (expicit SFL)
                                        SFLA
                            buy 2        buy 2
        */
        let txs = vec![
            TTx{t_day: 1, act: A::Buy, shares: gez!(10), price: gez!(1.0), af_name: "", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(5), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 1, act: A::Buy, shares: gez!(7), price: gez!(1.0), af_name: "B", ..def()}.x(),
            TTx{t_day: 40, act: A::Sell, shares: gez!(2), price: gez!(0.5), af_name: "", sfl: cadsfl(lez!(-1.0), false), ..def()}.x(),
            TTx{t_day: 40, act: A::Sfla, shares: gez!(1), price: gez!(0.5), af_name: "B", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "(R)", ..def()}.x(),
            TTx{t_day: 41, act: A::Buy, shares: gez!(2), price: gez!(1.0), af_name: "B", ..def()}.x(),
        ];

        let deltas = txs_to_delta_list_no_err(txs);
        validate_deltas(deltas, vec![
            TDt{post_st: TPSS{shares: gez!(10), all_shares: gez!(10), total_acb: sgez!(10.0), ..def()}, ..def()},                             // Buy in Default
            TDt{post_st: TPSS{shares: gez!(5), all_shares: gez!(15), acb_per_sh: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: sgez!(7.0), ..def()}, ..def()},                               // Buy in B
            TDt{post_st: TPSS{shares: gez!(8), all_shares: gez!(20), total_acb: sgez!(8.0), ..def()}, gain: sdec!(0), sfl: sndec!(-1.0), ..def()},             // SFL of 0.5 * 2 shares
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(20), total_acb: sgez!(7.5), ..def()}, ..def()},                               // Explicit adjust on B
            TDt{post_st: TPSS{shares: gez!(7), all_shares: gez!(22), total_acb: None, ..def()}, gain: None, ..def()}, // Buy in (R)
            TDt{post_st: TPSS{shares: gez!(9), all_shares: gez!(24), total_acb: sgez!(9.5), ..def()}, ..def()},                               // Buy in B
        ]);
    }
}