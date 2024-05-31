use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use time::Date;
use tracing::debug;

use crate::{
    portfolio::{bookkeeping::superficial_loss::{get_first_day_in_superficial_loss_period, get_last_day_in_superficial_loss_period}, Affiliate, SFLInput},
    util::decimal::{is_negative, GreaterEqualZeroDecimal, LessEqualZeroDecimal, PosDecimal}
};

use super::{CurrencyAndExchangeRate, Security, Tx, TxDelta};

type Error = String;
type Warning = String;

#[derive(Debug)]
struct SummaryRanges {
    latest_delta_in_summary_range_idx: usize,
    latest_summarizable_delta_idx: Option<usize>,
}

const GET_SUMMARY_RANGE_DELTA_INDICIES_WARN: &str = "No transactions in the summary period";

fn get_summary_range_delta_indicies(latest_date: Date, deltas: &Vec<TxDelta>)
    -> Option<SummaryRanges> {

    // Step 1: Find the latest Delta <= latest_date
    let mut latest_delta_in_summary_range_idx: Option<usize> = None;
    for (i, delta) in deltas.iter().enumerate() {
        if delta.tx.settlement_date > latest_date {
            break;
        }
        latest_delta_in_summary_range_idx = Some(i);
    }
    let latest_delta_in_summary_range_idx = match latest_delta_in_summary_range_idx {
        Some(v) => v,
        None => {
            return None;
        },
    };

    // Step 2: determine if any of the TXs within 30 days of latestDate are
    // superficial losses.
    // If any are, save the date 30 days prior to it (firstSuperficialLossPeriodDay)
    let latest_in_summary_tx = &deltas[latest_delta_in_summary_range_idx].tx;
    let latest_in_summary_date: Date = latest_in_summary_tx.settlement_date;
    let mut tx_in_summary_overlaps_superficial_loss = false;
    let mut first_superficial_loss_period_day = Date::from_calendar_date(3000, time::Month::January, 1).unwrap();
    // for _, delta := range deltas[latestDeltaInSummaryRangeIdx+1:] {
    for delta in &deltas[latest_delta_in_summary_range_idx + 1..] {
        if delta.is_superficial_loss() {
            first_superficial_loss_period_day = get_first_day_in_superficial_loss_period(delta.tx.settlement_date);
            tx_in_summary_overlaps_superficial_loss = latest_in_summary_date >= first_superficial_loss_period_day;
            if tx_in_summary_overlaps_superficial_loss {
                debug!(
                    "get_summary_range_delta_indicies: {} tx in {} settled on {} is in SFL period \
                    (starting {}) of tx settled on {} (SFL of {})",
                    latest_in_summary_tx.security, latest_in_summary_tx.affiliate.name(),
                    latest_in_summary_tx.settlement_date, first_superficial_loss_period_day,
                    delta.tx.settlement_date, *delta.sfl.as_ref().unwrap().superficial_loss,
                );
            }
            break
        }
    }

    // Step 3: Find the latest TX in the summary period that can't affect any
    // unsummarized superficial losses.
    let mut latest_summarizable_delta_idx: Option<usize> = None;
    if tx_in_summary_overlaps_superficial_loss {
        // Find the txs which we wanted to summarize, but can't because they can affect
        // this superficial loss' partial calculation.
        // This will be any tx within the 30 day period of the first superficial loss
        // after the summary boundary, but also every tx within the 30 period
        // of any superficial loss at the end of the summary range.
        let mut latest_summarizable_date: Option<Date> = None;
        for i in (0..=latest_delta_in_summary_range_idx).rev() {
            let delta = &deltas[i];
            if delta.tx.settlement_date< first_superficial_loss_period_day {
                latest_summarizable_delta_idx = Some(i);
                latest_summarizable_date = Some(delta.tx.settlement_date);
                break;
            }
            if delta.is_superficial_loss() {
                // We've encountered another superficial loss within the summary
                // range. This can be affected by previous txs, so we need to now push
                // up the period where we can't find any txs.
                first_superficial_loss_period_day = get_first_day_in_superficial_loss_period(delta.tx.settlement_date);
            }
        }
        debug!("   latestSummarizableDeltaIdx: {:?} ({:?})",
               latest_summarizable_delta_idx, latest_summarizable_date);
    } else {
        latest_summarizable_delta_idx = Some(latest_delta_in_summary_range_idx)
    }

    Some(SummaryRanges {
        latest_delta_in_summary_range_idx: latest_delta_in_summary_range_idx,
        latest_summarizable_delta_idx: latest_summarizable_delta_idx,
    })
}

const SHARE_BALANCE_ZERO_WARNING: &str = "Share balance at the end of the summarized period was zero";

fn make_simple_summary_txs(
    af: &Affiliate, deltas: &Vec<TxDelta>, latest_summarizable_delta_idx: usize)
    -> (Vec<Tx>, Vec<Warning>) {

    let tx = &deltas[latest_summarizable_delta_idx].tx;
    // All one TX. No capital gains yet.
    let sum_post_status = &deltas[latest_summarizable_delta_idx].post_status;
    if let Ok(share_balance) = PosDecimal::try_from(*sum_post_status.share_balance) {
        let summary_tx = Tx{
            security: tx.security.clone(),
            // Use same day for TradeDate and SettlementDate, since this is not
            // a real trade, and has no exchange rate to depend on the trade date.
            trade_date: tx.settlement_date,
            settlement_date: tx.settlement_date,
            action_specifics: super::TxActionSpecifics::Buy(
                super::BuyTxSpecifics {
                    shares: share_balance,
                    amount_per_share: match sum_post_status.total_acb {
                        Some(total_acb) => total_acb.div(share_balance),
                        None => GreaterEqualZeroDecimal::zero(),
                    },
                    commission: GreaterEqualZeroDecimal::zero(),
                    tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                    separate_commission_currency: None,
                }),
            memo: "Summary".to_string(),
            affiliate: af.clone(),
            read_index: 0, // This needs to be the first Tx in the list.
        };
        (vec![summary_tx], Vec::new())
    } else {
        (Vec::new(), vec![SHARE_BALANCE_ZERO_WARNING.to_string()])
    }
}

fn make_annual_gains_summary_txs(
    af: &Affiliate, deltas: &Vec<TxDelta>, latest_summarizable_delta_idx: usize)
    -> (Vec<Tx>, Vec<Warning>) {

    let mut warnings = Vec::<String>::new();
    let mut summary_period_txs = Vec::<Tx>::new();

    let mut yearly_cap_gains = HashMap::<i32, Decimal>::new();
    let mut latest_year_delta = HashMap::<i32, &TxDelta>::new();
    let first_year = deltas[0].tx.settlement_date.year();
    if !af.registered() {
        for delta in deltas[..latest_summarizable_delta_idx + 1].iter() {
            if delta.tx.affiliate != *af {
                continue;
            }
            let year = delta.tx.settlement_date.year();
            if !delta.capital_gain.unwrap_or(Decimal::ZERO).is_zero() {
                let prev_cap_gain = yearly_cap_gains.get(&year).map(|g| *g)
                    .unwrap_or(Decimal::ZERO);
                yearly_cap_gains.insert(year, prev_cap_gain + delta.capital_gain.unwrap());
            }
            latest_year_delta.insert(year, delta);
        }
    }

    let mut sorted_years_with_gains: Vec<i32> = yearly_cap_gains.keys().cloned().collect();
    sorted_years_with_gains.sort();

    let read_index: u32 = 0;

    let sum_post_status = &deltas[latest_summarizable_delta_idx].post_status;
    let base_acb_per_share = match sum_post_status.total_acb {
        Some(total_acb) => match PosDecimal::try_from(*sum_post_status.share_balance) {
            Ok(shares) => Some(total_acb.div(shares)),
            Err(_) => {
                warnings.push(SHARE_BALANCE_ZERO_WARNING.to_string());
                Some(GreaterEqualZeroDecimal::zero())
            },
        },
        None => None,
    };

    // Add length of sorted_years_with_gains to the share balance, as we'll sell one share per year
    // This will generally always be non-zero for non-registered affiliates
    let n_base_shares = sum_post_status.share_balance +
        GreaterEqualZeroDecimal::try_from(Decimal::from(sorted_years_with_gains.len())).unwrap();
    if let Ok(n_base_shares) = PosDecimal::try_from(*n_base_shares) {
        let tx = &deltas[latest_summarizable_delta_idx].tx;
        // Get the earliest year, and use Jan 1 of the previous year for the buy.
        let dt = Date::from_calendar_date(first_year - 1, time::Month::January, 1).unwrap();
        let setup_buy_sum_tx = Tx{
            security: tx.security.clone(),
            trade_date: dt.clone(),
            settlement_date: dt.clone(),
            action_specifics: super::TxActionSpecifics::Buy(
                super::BuyTxSpecifics {
                    shares: n_base_shares,
                    amount_per_share: match base_acb_per_share {
                        Some(aps) => aps,
                        None => GreaterEqualZeroDecimal::zero(),
                    },
                    commission: GreaterEqualZeroDecimal::zero(),
                    tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                    separate_commission_currency: None,
                }),
            memo: "Summary base (buy)".to_string(),
            affiliate: af.clone(),
            read_index: read_index,
        };

        summary_period_txs.push(setup_buy_sum_tx);
    }

    for year in sorted_years_with_gains {
        let gain_or_loss = yearly_cap_gains[&year];
        let (gain, loss) = if is_negative(&gain_or_loss) {
            (GreaterEqualZeroDecimal::zero(),
             GreaterEqualZeroDecimal::try_from(gain_or_loss * Decimal::NEGATIVE_ONE).unwrap())
        } else {
            (GreaterEqualZeroDecimal::try_from(gain_or_loss).unwrap(),
            GreaterEqualZeroDecimal::zero())
        };

        let tx = &latest_year_delta[&year].tx;
        let dt = Date::from_calendar_date(tx.settlement_date.year(), time::Month::January, 1).unwrap();
        let amount = match base_acb_per_share {
            Some(aps) => aps + gain,
            None => GreaterEqualZeroDecimal::zero(),
        };

        let summary_tx = Tx{
            security: tx.security.clone(),
            trade_date: dt,
            settlement_date: dt,
            action_specifics: super::TxActionSpecifics::Sell(
                super::SellTxSpecifics{
                    shares: PosDecimal::one(),
                    amount_per_share: amount,
                    commission: loss,
                    tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                    separate_commission_currency: None,
                    specified_superficial_loss: None,
                }
            ),
            memo: format!("{year} gain summary (sell)"),
            affiliate: af.clone(),
            read_index,
        };

        summary_period_txs.push(summary_tx);
    }

    (summary_period_txs, warnings)
}

/// Return a slice of Txs which can summarise all txs in `deltas` up to `latest_date`.
/// Multiple Txs might be returned if it is not possible to accurately summarise
/// in a single Tx without altering superficial losses (and preserving overall
/// capital gains?)
///
/// Note that `deltas` should provide all TXs for 60 days after latest_date, otherwise
/// the summary may become innacurate/problematic if a new TX were added within
/// that 60 day period after, and introduced a new superficial loss within 30 days
/// of the summary.
///
/// eg 1. (cannot be summarized)
/// 2021-11-05 BUY  1  @ 1.50
/// 2021-12-05 BUY  11 @ 1.50
/// 2022-01-01 SELL 10 @ 1.00
///
/// Return: summary Txs, user warnings
fn make_summary_txs(latest_date: Date, deltas: &Vec<TxDelta>,
                    split_annual_gains: bool)
    -> (Vec<Tx>, Vec<Warning>) {
    tracing::trace!("make_summary_txs: latest_date = {}", latest_date);

    let summary_range = match get_summary_range_delta_indicies(latest_date, deltas) {
        Some(v) => v,
        None => {
            return (Vec::new(), vec![GET_SUMMARY_RANGE_DELTA_INDICIES_WARN.to_string()]);
        },
    };

    tracing::trace!("make_summary_txs {summary_range:#?}");

    // Create a map of affiliate to its last summarizable delta index. Not all
    // affiliates will have one.
    let mut affil_last_summarizable_delta_idxs = HashMap::<Affiliate, usize>::new();
    // affils will be sorted alphabetically for determinism
    let mut affils = Vec::<Affiliate>::new();
    if let Some(latest_summarizable_delta_idx) = summary_range.latest_summarizable_delta_idx {
        for i in (0..=latest_summarizable_delta_idx).rev() {
            let af = &deltas[i].tx.affiliate;
            if !affil_last_summarizable_delta_idxs.contains_key(af) {
                affil_last_summarizable_delta_idxs.insert(af.clone(), i);
                affils.push(af.clone());
            }
        }
    }
    affils.sort_by(|a, b| a.id().partial_cmp(b.id()).unwrap());

    let mut summary_period_txs = Vec::<Tx>::new();
    let mut warnings = HashSet::<String>::new();
    for af in affils {
        let affil_last_summarizable_delta_idx = affil_last_summarizable_delta_idxs[&af];

        let (af_sum_txs, warns) = if split_annual_gains {
            make_annual_gains_summary_txs(&af, deltas, affil_last_summarizable_delta_idx)
        } else {
            make_simple_summary_txs(&af, deltas, affil_last_summarizable_delta_idx)
        };
        summary_period_txs.extend(af_sum_txs.into_iter());
        warnings.extend(warns.into_iter());
    }

    // Sort all of the txs. First, set their read_index, so that we
    // can tie-break. This will be unset after the sort, since these
    // read_indexes are bogus, and would conflict with those on the
    // existing non-summary Txs.
    for (i, tx) in summary_period_txs.iter_mut().enumerate() {
        tx.read_index = i as u32;
    }
    summary_period_txs.sort();
    for tx in summary_period_txs.iter_mut() {
        tx.read_index = 0;
    }

    // Find all unsummarizable Txs, and append them after the summary Txs
    let unsummarizable_deltas: Vec<&TxDelta> =
        if let Some(latest_summarizable_delta_idx) = summary_range.latest_summarizable_delta_idx {
            let first_unsumarizable_delta_idx = latest_summarizable_delta_idx + 1;
            deltas[first_unsumarizable_delta_idx..=
                   summary_range.latest_delta_in_summary_range_idx]
            .iter().collect()
        } else {
            // None of the deltas are summarizable
            deltas[..=summary_range.latest_delta_in_summary_range_idx]
            .iter().collect()
        };

    if unsummarizable_deltas.len() > 0 {
        warnings.insert(
            format!(
                "Some transactions to be summarized (between {} and {}) could not be due to superficial-loss conflicts",
                unsummarizable_deltas.first().unwrap().tx.trade_date.to_string(),
                unsummarizable_deltas.last().unwrap().tx.trade_date.to_string(),

            )
        );
    }
    tracing::trace!("make_summary_txs:   unsummarizable_deltas.len = {}", unsummarizable_deltas.len());
    for delta in unsummarizable_deltas {
        let mut unsum_tx = delta.tx.clone();
        match &delta.sfl {
            Some(sfl) => {
                // The proess of generating the deltas will create Sfla Tx/Deltas, so
                // if we emit these as a csv, we MUST convert all SFL sales to have
                // explicit superficial losses.
                // Copy, and add add SFL
                match &mut unsum_tx.action_specifics {
                    crate::portfolio::TxActionSpecifics::Sell(sell_specs) => {
                        sell_specs.specified_superficial_loss = Some(
                            SFLInput{
                                superficial_loss: LessEqualZeroDecimal::from(sfl.superficial_loss),
                                force: false }
                        );
                    },
                    _ => {
                        panic!("Superficial loss was not sell, but was {}", unsum_tx.action());
                    }
                }
            },
            None => (),
        }

        summary_period_txs.push(unsum_tx);
    }

    // Check for warning against insufficient time waited since
    // last summary day.
    {
        let today = crate::util::date::today_local();
        let last_summarizable_delta = &deltas[summary_range.latest_delta_in_summary_range_idx];
        // Find the very latest day that could possibly ever affect or be affected by
        // the last tx. This should be 60 days.
        let last_affecting_day = get_last_day_in_superficial_loss_period(
            get_last_day_in_superficial_loss_period(last_summarizable_delta.tx.settlement_date));
        if today <= last_affecting_day {
            warnings.insert(
                "The current date is such that new TXs could potentially alter how the \
                summary is created. You should wait 60 days after your latest \
                transaction within the summary period to generate the summary".to_string());
        }
    }

    (summary_period_txs, warnings.into_iter().collect())
}

pub struct CollectedSummaryData {
    pub txs: Vec<Tx>,
    // Warnings -> list of secs that encountered this warning
    pub warnings: HashMap<Warning, Vec<Security>>,
    // Security -> errors encountered (populated externally)
    // TODO delete this?
    #[deprecated]
    pub errors: HashMap<Security, Vec<Error>>,
}

/// Public-facing wrapper for make_summary_txs, handling
/// many securities at once.
///
/// TODO summarize annually generally. ie, have the amount bought and sold each year
/// be accurate, as well as the gains/loss.
pub fn make_aggregate_summary_txs(
    latest_date: Date, deltas_by_sec: &HashMap<Security, Vec<TxDelta>>,
    split_annual_gains: bool)
    -> CollectedSummaryData {

    let mut all_summary_txs = Vec::<Tx>::new();
    // Warnings -> list of secs that encountered this warning.
    let mut all_warnings = HashMap::<Warning, Vec<Security>>::new();

    let mut sorted_secs: Vec<&String> = deltas_by_sec.keys().collect();
    sorted_secs.sort();

    for sec in &sorted_secs {
        let deltas = &deltas_by_sec[*sec];
        let (summary_txs, warnings) = make_summary_txs(
            latest_date, deltas, split_annual_gains);
        // Add warnings to all_warnings
        for warning in warnings {
            if !all_warnings.contains_key(&warning) {
                all_warnings.insert(warning.clone(), Vec::new());
            }
            all_warnings.get_mut(&warning).unwrap().push((**sec).clone());
        }

        all_summary_txs.extend(summary_txs.into_iter());
    }

    CollectedSummaryData{ txs: all_summary_txs, warnings: all_warnings, errors: HashMap::new() }
}

// MARK: Tests
#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::{
        portfolio::{bookkeeping::txs_to_delta_list, testlib::TTx, SFLInput, Tx, TxDelta}, tracing::setup_tracing, util::{decimal::{GreaterEqualZeroDecimal, LessEqualZeroDecimal}, math::c_maybe_round_to_effective_cent}};
    use crate::util::date::pub_testlib::doy_date;
    use crate::testlib::assert_vecr_eq;

    use crate::gezdec as gez;
    use crate::lezdec as lez;
    use crate::portfolio::TxAction as A;

    use super::make_summary_txs;

    // Shortening alias
    fn def<T: Default>() -> T {
        Default::default()
    }

    const MATCHING_MEMO_PREFIX: &str = "TEST_MEMO_MATCHES:";

    fn matchable_memo(pattern: &str) -> String {
        MATCHING_MEMO_PREFIX.to_string() + pattern
    }

    fn make_sfla_tx_yd(year: u32, day_of_year: i64, amount: GreaterEqualZeroDecimal) -> Tx {
        let dt = doy_date(year, day_of_year);
        TTx{
            s_date: dt,
            act: crate::portfolio::TxAction::Sfla,
            shares: gez!(1),
            price: amount,
            memo: matchable_memo("Automatic SfL ACB adjustment.*"),
            ..TTx::default()
        }.x()
    }

    struct TSimpleSumTx {
        pub year: u32,
        pub doy: i64,
        pub shares: GreaterEqualZeroDecimal,
        pub amount: GreaterEqualZeroDecimal,
        pub af_name: &'static str,
    }

    impl TSimpleSumTx {
        /// eXpand
        pub fn x(&self) -> Tx {
            let dt = doy_date(self.year, self.doy);
            TTx{
                t_date: dt,
                s_date: dt,
                act: crate::portfolio::TxAction::Buy,
                shares: self.shares,
                price: self.amount,
                memo: "Summary".to_string(),
                af_name: self.af_name,
                ..TTx::default()
            }.x()
        }
    }

    impl Default for TSimpleSumTx {
        fn default() -> Self {
            Self {
                year: 0,
                doy: 0,
                shares: gez!(0),
                amount: gez!(0),
                af_name: "",
            }
        }
    }

    struct TSumBaseBuyTx {
        pub year: u32,
        pub shares: GreaterEqualZeroDecimal,
        pub amount: GreaterEqualZeroDecimal,
        pub af_name: &'static str,
    }

    impl TSumBaseBuyTx {
        pub fn x(&self) -> Tx {
            let dt = doy_date(self.year, 0);
            TTx{
                t_date: dt,
                s_date: dt,
                act: crate::portfolio::TxAction::Buy,
                shares: self.shares,
                price: self.amount,
                memo: "Summary base (buy)".to_string(),
                af_name: self.af_name,
                ..TTx::default()
            }.x()
        }
    }

    impl Default for TSumBaseBuyTx {
        fn default() -> Self {
            Self{
                year: 0,
                shares: gez!(0),
                amount: gez!(0),
                af_name: "",
            }
        }
    }

    struct TSumGainsTx {
        pub year: u32,
        pub acb_per_sh: GreaterEqualZeroDecimal,
        pub gain: Decimal,
        pub af_name: &'static str,
    }

    impl TSumGainsTx {
        pub fn x(&self) -> Tx {
            let dt = doy_date(self.year, 0);
            let (amount, commission) = if self.gain >= Decimal::ZERO {
                (GreaterEqualZeroDecimal::try_from(
                    *self.acb_per_sh + self.gain).unwrap(),
                GreaterEqualZeroDecimal::zero())
            } else {
                (self.acb_per_sh,
                 GreaterEqualZeroDecimal::try_from(
                    self.gain * Decimal::NEGATIVE_ONE).unwrap())
            };

            TTx{
                t_date: dt,
                s_date: dt,
                act: crate::portfolio::TxAction::Sell,
                shares: gez!(1),
                price: amount,
                comm: commission,
                memo: format!("{} gain summary (sell)", self.year),
                af_name: self.af_name,
                ..TTx::default()
            }.x()
        }
    }

    impl Default for TSumGainsTx {
        fn default() -> Self {
            Self{
                year: 0,
                acb_per_sh: gez!(0),
                gain: Decimal::ZERO,
                af_name: "",
            }
        }
    }

    fn txs_to_ok_delta_list(txs: &Vec<Tx>) -> Vec<TxDelta> {
        txs_to_delta_list(txs, None).unwrap_full_deltas()
    }

    fn validate_txs(exp_txs: &Vec<Tx>, actual_txs: &Vec<Tx>) {
        let mut fixed_exp_txs = Vec::<Tx>::new();

        if exp_txs.len() == actual_txs.len() {
            for i in 0..exp_txs.len() {
                let exp_tx = &exp_txs[i];
                let actual_tx = &actual_txs[i];

                // If the expected tx is trying to do a regex
                // match on the actual Tx's memo, fix that up, so
                // our assertion below passes or fails (usefully).
                let mut fixed_up_tx = exp_tx.clone();
                if exp_tx.memo.starts_with(MATCHING_MEMO_PREFIX) {
                    let re = regex::Regex::new(
                        exp_tx.memo.split_at(MATCHING_MEMO_PREFIX.len()).1).unwrap();
                    fixed_up_tx.memo = match re.is_match(actual_tx.memo.as_str()) {
                        true => actual_tx.memo.clone(),
                        false => format!("TEST_MEMO_MATCH_FAIL: actual: '{}' did not match {:?}",
                                         actual_tx.memo, re),
                    }
                }
                fixed_exp_txs.push(fixed_up_tx);
            }
        }

        assert_vecr_eq(&fixed_exp_txs, actual_txs);
    }

    fn cadsfl(superficial_loss: LessEqualZeroDecimal, force: bool) -> Option<SFLInput> {
        Some(crate::portfolio::SFLInput{
            superficial_loss: superficial_loss,
            force: force,
        })
    }

    #[test]
    fn test_summary_basic() {
        setup_tracing();
        // Ensure we don't think we're too close to the summary date.
        crate::util::date::set_todays_date_for_test(doy_date(3000, 1));

        // TEST: simple one tx to one summary
        let txs = vec![
            TTx{s_yr: 2021, s_doy: 4, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2021, doy: 4, shares: gez!(10), amount: gez!(1.2), ..def()}.x(), // commission is added to share ACB
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: nothing at all
        let txs = Vec::new();

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&Vec::new(), &summary_txs);

        // TEST: only after summary period
        let txs = vec![
            TTx{s_yr: 2022, s_doy: 4, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&txs, &summary_txs);

        // TEST: only after summary period, but there is a close superficial loss
        let txs = vec![
            TTx{s_yr: 2022, s_doy: 4, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: 41, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(), // SFL
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&txs, &summary_txs);

        // TEST: only after summary period, but there is a further superficial loss
        let txs = vec![
            TTx{s_yr: 2022, s_doy: 40, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: 41, act: A::Sell, shares: gez!(5), price: gez!(0.2), ..def()}.x(), // SFL
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&txs, &summary_txs);

        // TEST: only before period, and there are terminating superficial losses
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -2, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -1, shares: gez!(8), amount: gez!(1.45), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: present [ SELL ... 2 days || SFL, BUY ] past
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -2, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
            TTx{s_yr: 2022, s_doy: 2, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),    // Gain
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -1, shares: gez!(8), amount: gez!(1.45), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: present [ SFL ... 30 days || SELL(+), BUY ] past
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -2, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),               // Gain
            TTx{s_yr: 2022, s_doy: 30, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -1, shares: gez!(8), amount: gez!(1.2), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: present [ SFL ... 29 days || SELL(+), 1 day...  BUY ] past
        // The post SFL will influence the summarizable TXs
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -2, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),               // Gain
            TTx{s_yr: 2022, s_doy: 29, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -2, shares: gez!(10), amount: gez!(1.2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(), // Gain
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: present [ SFL ... 29 days || SELL(+), 0 days...  BUY ] past
        // The post SFL will influence the summarizable TXs
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -1, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),               // Gain
            TTx{s_yr: 2022, s_doy: 29, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TTx{s_yr: 2022, s_doy: -1, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(), // Gain
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ] past
        // Unsummarizable SFL will push back the summarizing window.
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -32, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -31, act: A::Buy, shares: gez!(2), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 29, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -32, shares: gez!(8), amount: gez!(1.25), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -31, act: A::Buy, shares: gez!(2), price: gez!(1), comm: gez!(2), ..def()}.x(),                    // ACB of 14 total after here.
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), sfl: cadsfl(lez!(-2.4), false), ..def()}.x(), // SFL of $2.4
            make_sfla_tx_yd(2022, -1, gez!(2.4)),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: present [ SFL ... 29 days || <mix of SFLs, BUYs> ] past
        // Unsummarizable SFL will push back the summarizing window.
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -71, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -70, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
            // unsummarizable below
            TTx{s_yr: 2022, s_doy: -45, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -31, act: A::Buy, shares: gez!(2), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -15, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),  // SFL
            // end of summary period
            TTx{s_yr: 2022, s_doy: 29, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -70, shares: gez!(8), amount: gez!(1.45), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -45, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),                                      // post ACB = 21.6
            TTx{s_yr: 2022, s_doy: -31, act: A::Buy, shares: gez!(2), price: gez!(1), comm: gez!(2), ..def()}.x(),                                      // post ACB = 25.6
            TTx{s_yr: 2022, s_doy: -15, act: A::Sell, shares: gez!(2), price: gez!(0.2), sfl: cadsfl(lez!(-2.4444444444444444444444444444), false), ..def()}.x(), // ACB = 22.755555556
            make_sfla_tx_yd(2022, -15, gez!(2.4444444444444444444444444444)),                                                                              // ACB of 25.2
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), sfl: cadsfl(lez!(-2.75), false), ..def()}.x(),                // ACB = 22.05
            make_sfla_tx_yd(2022, -1, gez!(2.75)),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: before and after: present [ SFL ... 25 days || ... 5 days, SFL, BUY ] past
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -6, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -5, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
            // end of summary period
            TTx{s_yr: 2022, s_doy: 26, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -5, shares: gez!(8), amount: gez!(1.45), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: before and after: present [ SFL ... 2 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -34, act: A::Buy, shares: gez!(1), price: gez!(1), comm: gez!(1), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -33, act: A::Buy, shares: gez!(9), price: gez!(1), comm: gez!(1), ..def()}.x(),
            // unsummarizable below
            TTx{s_yr: 2022, s_doy: -20, act: A::Buy, shares: gez!(4), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -2, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
            TTx{s_yr: 2022, s_doy: -1, act: A::Buy, shares: gez!(2), price: gez!(0.2), comm: gez!(2), ..def()}.x(),
            // end of summary period
            TTx{s_yr: 2022, s_doy: 2, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 3, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -33, shares: gez!(10), amount: gez!(1.2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -20, act: A::Buy, shares: gez!(4), price: gez!(1), comm: gez!(2), ..def()}.x(),                                     // ACB = 18
            TTx{s_yr: 2022, s_doy: -2, act: A::Sell, shares: gez!(2), price: gez!(0.2), sfl: cadsfl(lez!(-2.1714285714285714285714285714), false), ..def()}.x(), // ACB = 15.428571429
            make_sfla_tx_yd(2022, -2, gez!(2.1714285714285714285714285714)),
            TTx{s_yr: 2022, s_doy: -1, act: A::Buy, shares: gez!(2), price: gez!(0.2), comm: gez!(2), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: before and after: present [ SFL ... 30 days || BUY, SFL ... 20 days, BUY ... 10 days, BUY ] past
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -33, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -20, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -2, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
            TTx{s_yr: 2022, s_doy: -1, act: A::Buy, shares: gez!(2), price: gez!(0.6), comm: gez!(2), ..def()}.x(),
            // end of summary period
            TTx{s_yr: 2022, s_doy: 30, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2022, doy: -1, shares: gez!(20), amount: gez!(1.34), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: No shares left in summary.
        let txs = vec![
            TTx{s_yr: 2021, s_doy: 4, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2021, s_doy: 4, act: A::Sell, shares: gez!(10), price: gez!(1), ..def()}.x(),
        ];
        let exp_summary_txs = Vec::new();

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);

        // TEST: No shares left in summarizable region
        let txs = vec![
            TTx{s_yr: 2022, s_doy: -33, act: A::Buy, shares: gez!(10), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -33, act: A::Sell, shares: gez!(10), price: gez!(2), ..def()}.x(), // Gain
            // unsummarizable below
            TTx{s_yr: 2022, s_doy: -20, act: A::Buy, shares: gez!(4), price: gez!(1), comm: gez!(2), ..def()}.x(),
            // end of summary period
            TTx{s_yr: 2022, s_doy: 2, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 3, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let exp_summary_txs = vec![
            TTx{s_yr: 2022, s_doy: -20, act: A::Buy, shares: gez!(4), price: gez!(1), comm: gez!(2), ..def()}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, false);
        assert_eq!(warnings.len(), 2);
        validate_txs(&exp_summary_txs, &summary_txs);
    }

    #[test]
    fn test_summary_year_splits() {
        // Ensure we don't think we're too close to the summary date.
        crate::util::date::set_todays_date_for_test(doy_date(3000, 1));

        // TEST: present [ SFL ... 29 days || SFL, 29 days... BUY, 1 day... BUY ... ] past
        // Unsummarizable SFL will push back the summarizing window.
        let txs = vec![
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 31, act: A::Sell, shares: gez!(1), price: gez!(2), ..def()}.x(),               // GAIN
            TTx{s_yr: 2020, s_doy: 100, act: A::Sell, shares: gez!(1), price: gez!(0.9), ..def()}.x(),            // LOSS
            TTx{s_yr: 2021, s_doy: 100, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),            // LOSS
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 29, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(),             // SFL
            TTx{s_yr: 2022, s_doy: 31, act: A::Buy, shares: gez!(1), price: gez!(2), comm: gez!(2), ..def()}.x(), // Causes SFL
        ];
        let summary_acb = gez!(1.25);
        let exp_summary_txs = vec![
            TSumBaseBuyTx{year: 2017, shares: gez!(14), amount: summary_acb, ..def()}.x(), // shares = final shares (12) + N years with gains (2)
            TSumGainsTx{year: 2020, acb_per_sh: summary_acb, gain: dec!(0.4), ..def()}.x(),
            TSumGainsTx{year: 2021, acb_per_sh: summary_acb, gain: dec!(-2.1), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: -1, act: A::Sell, shares: gez!(2), price: gez!(0.2), ..def()}.x(), // SFL
        ];

        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, true);
        assert_eq!(warnings.len(), 1);
        validate_txs(&exp_summary_txs, &summary_txs);
    }

    #[test]
    fn test_multi_affiliate_summary() {
        // Ensure we don't think we're too close to the summary date.
        crate::util::date::set_todays_date_for_test(doy_date(3000, 1));

        // Case: Test basic only buys for each affiliate.
        let txs = vec![
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(4), price: gez!(1), comm: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 31, act: A::Buy, shares: gez!(3), price: gez!(1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 29, act: A::Buy, shares: gez!(5), price: gez!(1), comm: gez!(2), af_name: "B", ..def()}.x(),
        ];

        let gez_10_o_8 = gez!(1.25); // 10.0 / 8.0
        let gez_9_7_o_7 = gez!(1.3857142857142857142857142857); // 9.7 / 7.0

        // Note these are sorted alphabetically to tiebreak between affiliates
        let exp_summary_txs = vec![
            TSumBaseBuyTx{year: 2017, shares: gez!(9), amount: gez!(1.4444444444444444444444444444)/*13.0 / 9.0*/, af_name: "B"}.x(),
            TSumBaseBuyTx{year: 2017, shares: gez!(8), amount: gez_10_o_8, af_name: ""}.x(),
            // Registered accounts use 0 rather than NaN in the summary
            TSumBaseBuyTx{year: 2017, shares: gez!(3), amount: gez!(0), af_name: "(R)"}.x(),
        ];

        let deltas = txs_to_ok_delta_list(&txs);

        let (summary_txs, warnings) = make_summary_txs(doy_date(2022, -1), &deltas, true);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Test capital gains in multiple years, different between affiliates.
        let txs = vec![
            // Buys
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 31, act: A::Buy, shares: gez!(6), price: gez!(1.2), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            // Sells
            TTx{s_yr: 2019, s_doy: 5, act: A::Sell, shares: gez!(1), price: gez!(2), ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 6, act: A::Sell, shares: gez!(1), price: gez!(2.1), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 7, act: A::Sell, shares: gez!(1), price: gez!(2.2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2021, s_doy: 7, act: A::Sell, shares: gez!(1), price: gez!(2.3), ..def()}.x(),
            TTx{s_yr: 2022, s_doy: 7, act: A::Sell, shares: gez!(1), price: gez!(2.4), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2022, s_doy: 8, act: A::Sell, shares: gez!(1), price: gez!(2.5), af_name: "(R)", ..def()}.x(),
        ];

        let def_share_acb = gez_10_o_8;
        let b_share_acb = gez_9_7_o_7;
        let exp_summary_txs = vec![
            TSumBaseBuyTx{year: 2017, shares: gez!(7), amount: b_share_acb, af_name: "B", ..def()}.x(),
            TSumBaseBuyTx{year: 2017, shares: gez!(8), amount: def_share_acb, ..def()}.x(),
            TSumBaseBuyTx{year: 2017, shares: gez!(4), amount: gez!(0), af_name: "(R)", ..def()}.x(), // No gains on (R), so only the base

            TSumGainsTx{year: 2019, acb_per_sh: b_share_acb, gain: dec!(2.1) - *b_share_acb, af_name: "B", ..def()}.x(),
            TSumGainsTx{year: 2019, acb_per_sh: def_share_acb, gain: dec!(2.0) - *def_share_acb, ..def()}.x(),
            TSumGainsTx{year: 2021, acb_per_sh: def_share_acb, gain: dec!(2.3) - *def_share_acb, ..def()}.x(),
            TSumGainsTx{year: 2022, acb_per_sh: b_share_acb, gain: dec!(2.4) - *b_share_acb, af_name: "B", ..def()}.x(),
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2024, -1), &deltas, true);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: only some affiliates have gains (registered & non-registered)
        let txs = vec![
            // Buys
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 31, act: A::Buy, shares: gez!(6), price: gez!(1.2), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            // Sells
            TTx{s_yr: 2019, s_doy: 5, act: A::Sell, shares: gez!(1), price: gez!(2), ..def()}.x(),
        ];

        let def_share_acb = gez_10_o_8;
        let b_share_acb = gez_9_7_o_7;
        let exp_summary_txs = vec![
            TSumBaseBuyTx{year: 2017, shares: gez!(7), amount: b_share_acb, af_name: "B", ..def()}.x(),
            TSumBaseBuyTx{year: 2017, shares: gez!(8), amount: def_share_acb, ..def()}.x(),
            TSumBaseBuyTx{year: 2017, shares: gez!(6), amount: gez!(0), af_name: "(R)", ..def()}.x(), // No gains on (R), so only the base

            TSumGainsTx{year: 2019, acb_per_sh: def_share_acb, gain: dec!(2.0) - *def_share_acb, ..def()}.x(),
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2024, -1), &deltas, true);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Simple summary, where some affiliates have sells
        let txs = vec![
            // Buys
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 31, act: A::Buy, shares: gez!(6), price: gez!(1.2), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 40, act: A::Buy, shares: gez!(5), price: gez!(1.3), comm: gez!(2), af_name: "B (R)", ..def()}.x(),
            // Sells
            TTx{s_yr: 2020, s_doy: 5, act: A::Sell, shares: gez!(2), price: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 6, act: A::Sell, shares: gez!(3), price: gez!(2), af_name: "B (R)", ..def()}.x(),
        ];

        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2018, doy: 30, shares: gez!(8), amount: gez_10_o_8, ..def()}.x(),
            TSimpleSumTx{year: 2019, doy: 31, shares: gez!(6), amount: gez!(0), af_name: "(R)", ..def()}.x(),
            TSimpleSumTx{year: 2020, doy: 5, shares: gez!(5), amount: gez_9_7_o_7, af_name: "B", ..def()}.x(),
            TSimpleSumTx{year: 2020, doy: 6, shares: gez!(2), amount: gez!(0), af_name: "B (R)", ..def()}.x(),
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2024, -1), &deltas, false /* year gains */);
        assert_vecr_eq(&warnings, &Vec::new());
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Some affiliates net zero shares at the end
        let txs = vec![
            // Buys
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 31, act: A::Buy, shares: gez!(6), price: gez!(1.2), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            // Sells
            TTx{s_yr: 2019, s_doy: 5, act: A::Sell, shares: gez!(7), price: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2019, s_doy: 5, act: A::Sell, shares: gez!(6), price: gez!(2), af_name: "(R)", ..def()}.x(),
        ];

        let b_share_acb = gez_9_7_o_7; // 9.7 / 7.0
        let exp_summary_txs = vec![
            TSumBaseBuyTx{year: 2017, shares: gez!(1), amount: gez!(0), af_name: "B", ..def()}.x(),
            TSumBaseBuyTx{year: 2017, shares: gez!(8), amount: gez!(1.25) /* 10.0 / 8 */ , ..def()}.x(),

            TSumGainsTx{year: 2019, acb_per_sh: gez!(0),
                        gain: crate::util::math::maybe_round_to_effective_cent((dec!(2.0) - *b_share_acb) * dec!(7.0)),
                        af_name: "B", ..def()}.x(),
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2024, -1), &deltas, true);
        assert_eq!(warnings.len(), 1); // zero warning
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Superficial losses in one affiliate, and another affiliate with zero Txs
        //            before the summarizable range.
        let txs = vec![
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 5, act: A::Sell, shares: gez!(2), price: gez!(2), af_name: "B", ..def()}.x(),
            // ^^ Summarizable period ^^
            TTx{s_yr: 2020, s_doy: 101, act: A::Buy, shares: gez!(2), price: gez!(1), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(1), af_name: "C", ..def()}.x(),
            // ^^ Requested summary period ^^
            TTx{s_yr: 2020, s_doy: 105, act: A::Sell, shares: gez!(2), price: gez!(0.9), sfl: cadsfl(lez!(-0.751020), false), af_name: "B", ..def()}.x(),
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2018, doy: 30, shares: gez!(8), amount: gez_10_o_8, ..def()}.x(),
            TSimpleSumTx{year: 2020, doy: 5, shares: gez!(5), amount: gez_9_7_o_7, af_name: "B", ..def()}.x(),
            // ^^ Summarizable period ^^
            TTx{s_yr: 2020, s_doy: 101, act: A::Buy, shares: gez!(2), price: gez!(1), af_name: "B", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(1), af_name: "C", ..def()}.x(),
            // ^^ Requested summary period ^^
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2020, 104), &deltas, false /* year gains*/);
        assert_eq!(warnings.len(), 1); // zero warning
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Superficial loss after summary period, and presence of registered
        //       affiliate (where all of their Deltas have a SuperficialLoss of NaN)
        let txs = vec![
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 59, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 60, act: A::Sell, shares: gez!(2), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            // ^^ Summarizable period ^^
            TTx{s_yr: 2020, s_doy: 102, act: A::Sell, shares: gez!(3), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(2), ..def()}.x(),
            // ^^ Requested summary period ^^
            TTx{s_yr: 2020, s_doy: 105, act: A::Sell, shares: gez!(2), price: gez!(0.9), sfl: cadsfl(lez!(-1.2), false), ..def()}.x(),
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2020, doy: 59, shares: gez!(6), amount: gez_10_o_8, ..def()}.x(),
            TSimpleSumTx{year: 2020, doy: 60, shares: gez!(5), amount: gez!(0), af_name: "(R)", ..def()}.x(),
            // ^^ Summarizable period ^^
            TTx{s_yr: 2020, s_doy: 102, act: A::Sell, shares: gez!(3), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(2), ..def()}.x(),
            // ^^ Requested summary period ^^
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2020, 104), &deltas, false /* year gains*/);
        assert_eq!(warnings.len(), 1); // zero warning
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Superficial loss after summary period, and presence of registered
        //       affiliate (where all of their Deltas have a SuperficialLoss of NaN) sales
        //            at least every 30 days until beginning of time. (verifies that the
        //            summarizable period doesn't keep getting pushed backwards).
        let txs = vec![
            TTx{s_yr: 2020, s_doy: 50, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 51, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 59, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 60, act: A::Sell, shares: gez!(1), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 70, act: A::Sell, shares: gez!(1), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            // ^^ Summarizable period ^^
            TTx{s_yr: 2020, s_doy: 85, act: A::Sell, shares: gez!(3), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(2), ..def()}.x(),
            // ^^ Requested summary period ^^
            TTx{s_yr: 2020, s_doy: 105, act: A::Sell, shares: gez!(2), price: gez!(0.9), sfl: cadsfl(lez!(-1.2), false), ..def()}.x(),
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2020, doy: 59, shares: gez!(6), amount: gez_10_o_8, ..def()}.x(),
            TSimpleSumTx{year: 2020, doy: 70, shares: gez!(5), amount: gez!(0), af_name: "(R)", ..def()}.x(),
            // ^^ Summarizable period ^^
            TTx{s_yr: 2020, s_doy: 85, act: A::Sell, shares: gez!(3), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(2), ..def()}.x(),
            // ^^ Requested summary period ^^
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2020, 104), &deltas, false /* year gains*/);
        assert_eq!(warnings.len(), 1); // zero warning
        validate_txs(&exp_summary_txs, &summary_txs);

        // Case: Only registered sales after summary period. Verify not treated as
        //            superficial losses (because their SuperficialLoss is NaN).
        let txs = vec![
            TTx{s_yr: 2018, s_doy: 30, act: A::Buy, shares: gez!(8), price: gez!(1), comm: gez!(2), ..def()}.x(),
            TTx{s_yr: 2018, s_doy: 31, act: A::Buy, shares: gez!(7), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 59, act: A::Sell, shares: gez!(2), price: gez!(2), ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 60, act: A::Sell, shares: gez!(2), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 102, act: A::Sell, shares: gez!(3), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
            TTx{s_yr: 2020, s_doy: 103, act: A::Buy, shares: gez!(3), price: gez!(2), ..def()}.x(),
            // ^^ Requested summary period ^^
            TTx{s_yr: 2020, s_doy: 105, act: A::Sell, shares: gez!(2), price: gez!(1.1), comm: gez!(2), af_name: "(R)", ..def()}.x(),
        ];
        let exp_summary_txs = vec![
            TSimpleSumTx{year: 2020, doy: 102, shares: gez!(2), amount: gez!(0), af_name: "(R)", ..def()}.x(),
            TSimpleSumTx{year: 2020, doy: 103, shares: gez!(9), amount: gez!(1.5), ..def()}.x(),
            // ^^ Summarizable period ^^
            // ^^ Requested summary period ^^
        ];
        let deltas = txs_to_ok_delta_list(&txs);
        let (summary_txs, warnings) = make_summary_txs(doy_date(2020, 104), &deltas, false /* year gains */);
        assert_vecr_eq(&warnings, &Vec::new()); // zero warning
        validate_txs(&exp_summary_txs, &summary_txs);
    }

}