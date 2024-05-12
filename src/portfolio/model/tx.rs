use std::fmt::Display;

use rust_decimal::Decimal;
use time::Date;

use crate::{
    portfolio::{csv_common::CsvCol, Affiliate},
    util::decimal::{GreaterEqualZeroDecimal, LessEqualZeroDecimal, PosDecimal}};

use super::currency::{Currency, CurrencyAndExchangeRate};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TxAction{
    Buy,
    Sell,
    Roc, // Return of capital
    Sfla, // Superficial loss ACB adjustment
}

impl TxAction {
    fn pretty_str(&self) -> &str {
        match self {
            TxAction::Buy => "Buy",
            TxAction::Sell => "Sell",
            TxAction::Roc => "RoC",
            TxAction::Sfla => "SfLA",
        }
    }
}

impl Display for TxAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty_str())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SFLInput {
	pub superficial_loss: LessEqualZeroDecimal,
	pub force: bool
}

impl SFLInput {
    pub fn req_from_dec(v: Decimal, force: bool) -> SFLInput {
        SFLInput{ superficial_loss: LessEqualZeroDecimal::try_from(v).unwrap(), force: force }
    }
}

// This Transaction type is flat, and designed to absorb input from a uniform
// CSV/table input. It is fairly unconstrained, and as such, should be converted
// to the Tx type for processing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CsvTx {
    pub security: Option<String>,
    pub trade_date: Option<Date>,
    pub settlement_date: Option<Date>,
    pub action: Option<TxAction>,
    pub shares: Option<Decimal>,
    pub amount_per_share: Option<Decimal>,
    pub commission: Option<Decimal>,

    pub tx_currency: Option<Currency>,
    pub tx_curr_to_local_exchange_rate: Option<Decimal>,

    pub commission_currency: Option<Currency>,
    pub commission_curr_to_local_exchange_rate: Option<Decimal>,

    pub memo: Option<String>,
    pub affiliate: Option<Affiliate>,

    // More commonly optional fields/columns

    // The total superficial loss for the transaction, as explicitly
    // specified by the user. May be cross-validated against calculated SFL to emit
    // warnings. If specified, the user is also required to specify one or more
    // SfLA Txs following this one, accounting for all shares experiencing the loss.
    // NOTE: This is always a negative (or zero) value in CAD, so that it matches the
    // displayed value
    pub specified_superficial_loss: Option<SFLInput>,

    // The absolute order in which the Tx was read from file or entered.
    // Used as a tiebreak in sorting.
    pub read_index: u32,
}

impl Default for CsvTx {
    fn default() -> Self {
        CsvTx{
            security: None,
            trade_date: None,
            settlement_date: None,
            action: None,
            shares: None,
            amount_per_share: None,
            commission: None,
            tx_currency: None,
            tx_curr_to_local_exchange_rate: None,
            commission_currency: None,
            commission_curr_to_local_exchange_rate: None,
            memo: None,
            affiliate: None,
            specified_superficial_loss: None,
            read_index: 0,
        }
    }
}

impl PartialOrd for CsvTx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let date_cmp = self.settlement_date.cmp(&other.settlement_date);
        match date_cmp {
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater => Some(date_cmp),
            std::cmp::Ordering::Equal => Some(self.read_index.cmp(&other.read_index)),
        }
    }
}

impl Ord for CsvTx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuyTxSpecifics {
    pub shares: PosDecimal,
    pub amount_per_share: GreaterEqualZeroDecimal,
    pub commission: GreaterEqualZeroDecimal,

    pub tx_currency_and_rate: CurrencyAndExchangeRate,
    pub separate_commission_currency: Option<CurrencyAndExchangeRate>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SellTxSpecifics {
    pub shares: PosDecimal,
    pub amount_per_share: GreaterEqualZeroDecimal,
    pub commission: GreaterEqualZeroDecimal,

    pub tx_currency_and_rate: CurrencyAndExchangeRate,
    pub separate_commission_currency: Option<CurrencyAndExchangeRate>,

    pub specified_superficial_loss: Option<SFLInput>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RocTxSpecifics {
    pub amount_per_held_share: GreaterEqualZeroDecimal,
    pub tx_currency_and_rate: CurrencyAndExchangeRate,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SflaTxSpecifics {
    pub total_amount: PosDecimal,
    // There is no currency here, because this is _always_ in CAD.
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TxActionSpecifics {
    Buy(BuyTxSpecifics),
    Sell(SellTxSpecifics),
    Roc(RocTxSpecifics),
    Sfla(SflaTxSpecifics),
}

impl TxActionSpecifics {
    pub fn action(&self) -> TxAction {
        match self {
            TxActionSpecifics::Buy(_) => TxAction::Buy,
            TxActionSpecifics::Sell(_) => TxAction::Sell,
            TxActionSpecifics::Roc(_) => TxAction::Roc,
            TxActionSpecifics::Sfla(_) => TxAction::Sfla,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tx {
    pub security: String,
    pub trade_date: Date,
    pub settlement_date: Date,
    pub action_specifics: TxActionSpecifics,
    pub memo: String,
    pub affiliate: Affiliate,
    pub read_index: u32,
}

impl Tx {
    pub fn buy_specifics(&self) -> Result<&BuyTxSpecifics, ()>{
        match &self.action_specifics {
            TxActionSpecifics::Buy(specs) => Ok(specs),
            _ => Err(()),
        }
    }
}

fn get_valid_exchange_rate(curr_col_name: &str, found_curr: &Option<Currency>,
                           fx_col_name: &str, found_fx: &Option<Decimal>,
    ) -> Result<Option<CurrencyAndExchangeRate>, String> {
    if found_curr.is_none() && found_fx.is_none() {
        Ok(None)
    } else {
        let curr = found_curr.clone().ok_or(
            format!("\"{fx_col_name}\" specified but \"{curr_col_name}\" not found"))?;
        if curr == Currency::default() && found_fx.is_none() {
            // The rate here is implicitly 1, so the rate isn't required.
            // If rate is provided, we continue below to check its validity.
            return Ok(Some(CurrencyAndExchangeRate::default()));
        }
        let rate = found_fx.ok_or(
            format!("\"{curr_col_name}\" specified but \"{fx_col_name}\" not found"))?;
        Ok(Some(CurrencyAndExchangeRate::try_new(curr, PosDecimal::try_from(rate).map_err(
            |_| format!("\"{fx_col_name}\" must be a positive value"))?)?))
    }
}

// For now, just return BuyTxSpecifics, since it is a strict subset of
// SellTxSpecifics
fn buy_or_sell_common_attrs_from_csv_tx(csv_tx: &CsvTx, buy_or_sell: &str) -> Result<BuyTxSpecifics, String> {
    let shares = csv_tx.shares.ok_or("\"shares\" not found")?;
    let amount_per_share = csv_tx.amount_per_share.ok_or("\"amount/share\" not found")?;
    let commission = csv_tx.commission.unwrap_or(Decimal::ZERO);

    let curr_and_rate = get_valid_exchange_rate(
        CsvCol::TX_CURR, &csv_tx.tx_currency,
        CsvCol::TX_FX, &csv_tx.tx_curr_to_local_exchange_rate
    )?.unwrap_or_else(|| CurrencyAndExchangeRate::default());

    let comm_curr_and_rate = get_valid_exchange_rate(
        CsvCol::COMMISSION_CURR, &csv_tx.commission_currency,
        CsvCol::COMMISSION_FX, &csv_tx.commission_curr_to_local_exchange_rate
    )?;

    let specifics = BuyTxSpecifics{
        shares: PosDecimal::try_from(shares).map_err(|_| format!(
            "{buy_or_sell} shares must be a positive value"))?,
        amount_per_share: GreaterEqualZeroDecimal::try_from(amount_per_share).map_err(|_| format!(
            "{buy_or_sell} amount/share must not be negative"))?,
        commission: GreaterEqualZeroDecimal::try_from(commission).map_err(|_| format!(
            "{buy_or_sell} comission must not be negative"))?,
        tx_currency_and_rate: curr_and_rate,
        separate_commission_currency: comm_curr_and_rate,
    };
    Ok(specifics)
}

impl TryFrom<CsvTx> for Tx {
    type Error = String;

    fn try_from(csv_tx: CsvTx) -> Result<Self, Self::Error> {
        if csv_tx.action.is_none() {
            return Err("\"action\" not specified".to_string());
        }

        let act_specs = match csv_tx.action.unwrap() {
            TxAction::Buy => {
                let common = buy_or_sell_common_attrs_from_csv_tx(&csv_tx, "Buy")?;
                TxActionSpecifics::Buy(common)
            },
            TxAction::Sell => {
                let common = buy_or_sell_common_attrs_from_csv_tx(&csv_tx, "Buy")?;
                TxActionSpecifics::Sell(SellTxSpecifics{
                    shares: common.shares,
                    amount_per_share: common.amount_per_share,
                    commission: common.commission,
                    tx_currency_and_rate: common.tx_currency_and_rate,
                    separate_commission_currency: common.separate_commission_currency,
                    specified_superficial_loss: csv_tx.specified_superficial_loss,
                })
            },
            TxAction::Roc => {
                let amount_per_share = csv_tx.amount_per_share.ok_or(
                    format!("RoC \"{}\" not found", CsvCol::AMOUNT_PER_SHARE))?;

                let amount = GreaterEqualZeroDecimal::try_from(
                    amount_per_share).map_err(|_| format!(
                        "RoC amount per share must not be negative. Found {}", amount_per_share))?;

                let curr_and_rate = get_valid_exchange_rate(
                    CsvCol::TX_CURR, &csv_tx.tx_currency,
                    CsvCol::TX_FX, &csv_tx.tx_curr_to_local_exchange_rate
                )?.unwrap_or_else(|| CurrencyAndExchangeRate::default());

                TxActionSpecifics::Roc(RocTxSpecifics{
                    amount_per_held_share: amount,
                    tx_currency_and_rate: curr_and_rate,
                })
            },
            TxAction::Sfla => {
                let amount_per_share = csv_tx.amount_per_share.ok_or(
                    format!("SfLA \"{}\" not found", CsvCol::AMOUNT_PER_SHARE))?;
                let shares = csv_tx.shares.ok_or(
                    format!("SfLA \"{}\" not found", CsvCol::SHARES))?;

                let total = PosDecimal::try_from(
                    amount_per_share * shares).map_err(
                        |_| format!(
                            "SfLA ({}) x ({}) must be positive ({} x {})",
                            CsvCol::AMOUNT_PER_SHARE, CsvCol::SHARES,
                            amount_per_share, shares))?;
                TxActionSpecifics::Sfla(SflaTxSpecifics{
                    total_amount: total,
                })
            },
        };

        let not_found_err = |col_name| -> String {
            format!("\"{col_name}\" not found")
        };

        let tx = Tx{
            security: csv_tx.security.ok_or_else(|| not_found_err(CsvCol::SECURITY))?,
            trade_date: csv_tx.trade_date.ok_or_else(|| not_found_err(CsvCol::TRADE_DATE))?,
            settlement_date: csv_tx.settlement_date.ok_or_else(|| not_found_err(CsvCol::SETTLEMENT_DATE))?,
            action_specifics: act_specs,
            memo: csv_tx.memo.unwrap_or_else(|| String::new()),
            affiliate: csv_tx.affiliate.unwrap_or_else(|| Affiliate::default()),
            read_index: csv_tx.read_index,
        };
        if tx.security.is_empty() {
            return Err(format!("\"{}\" was empty", CsvCol::SECURITY))
        }
        Ok(tx)
    }
}

impl PartialOrd for Tx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let date_cmp = self.settlement_date.cmp(&other.settlement_date);
        match date_cmp {
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater => Some(date_cmp),
            std::cmp::Ordering::Equal => Some(self.read_index.cmp(&other.read_index)),
        }
    }
}

impl Ord for Tx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::Date;

    use crate::pdec;
    use crate::portfolio::{Affiliate, Currency, CurrencyAndExchangeRate, SellTxSpecifics, SflaTxSpecifics};
    use crate::testlib::assert_big_struct_eq;
    use crate::util::decimal::{GreaterEqualZeroDecimal, PosDecimal};
    use crate::{testlib::assert_vec_eq, util::date::parse_standard_date};
    use crate::util::date;

    use super::{BuyTxSpecifics, CsvTx, RocTxSpecifics, SFLInput, Tx, TxAction};

    pub const DEFAULT_SECURITY: &str = "FOO";

    pub fn tx_default() -> CsvTx {
        CsvTx { security: Some(DEFAULT_SECURITY.to_string()),
             trade_date: Some(Date::MIN),
             settlement_date: Some(Date::MIN),
             action: Some(TxAction::Buy),
             shares: Some(dec!(1)),
             amount_per_share: Some(dec!(0)),
            ..CsvTx::default()
            }
    }

    fn pos(d: Decimal) -> PosDecimal {
        PosDecimal::try_from(d).unwrap()
    }

    fn gez(d: Decimal) -> GreaterEqualZeroDecimal {
        GreaterEqualZeroDecimal::try_from(d).unwrap()
    }

    #[test]
    fn test_tx_order() {
        let doy_date = |day| { date::pub_testlib::doy_date(2024, day) };
        let sdoy_date = |day| { Some(doy_date(day)) };

        let mut csv_txs = vec![
            CsvTx{settlement_date: sdoy_date(4), read_index: 2, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(5), read_index: 1, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(2), read_index: 4, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(4), read_index: 3, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(1), read_index: 5, ..tx_default()},
        ];
        let mut txs: Vec<Tx> = csv_txs.iter().map(|c_tx| {
            Tx::try_from(c_tx.clone()).unwrap() }).collect();

        let exp_csv_txs = vec![
            CsvTx{settlement_date: sdoy_date(1), read_index: 5, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(2), read_index: 4, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(4), read_index: 2, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(4), read_index: 3, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(5), read_index: 1, ..tx_default()},
        ];
        let exp_txs: Vec<Tx> = exp_csv_txs.iter().map(|c_tx| {
            Tx::try_from(c_tx.clone()).unwrap() }).collect();

        csv_txs.sort();
        assert_vec_eq(csv_txs, exp_csv_txs);

        // Tx should have the same order
        txs.sort();
        assert_vec_eq(txs, exp_txs);
    }

    fn fully_populated_csvtx(action: TxAction) -> CsvTx {
        CsvTx{
            security: Some("FOO".to_string()),
            trade_date: Some(parse_standard_date("2022-10-20").unwrap()),
            settlement_date: Some(parse_standard_date("2022-10-21").unwrap()),
            action: Some(action),
            shares: Some(dec!(123.1)),
            amount_per_share: Some(dec!(10.1)),
            commission: Some(dec!(1.01)),
            tx_currency: Some(Currency::usd()),
            tx_curr_to_local_exchange_rate: Some(dec!(1.21)),
            commission_currency: Some(Currency::new("EUR")),
            commission_curr_to_local_exchange_rate: Some(dec!(2.01)),
            memo: Some("A memo".to_string()),
            affiliate: Some(Affiliate::default_registered()),
            // Ignored for buy
            specified_superficial_loss: Some(SFLInput::req_from_dec(dec!(-2.5), false)),
            read_index: 5,
        }
    }

    fn barebones_valid_sample_csvtx(action: TxAction) -> CsvTx {
        CsvTx{
            security: Some("FOO".to_string()),
            trade_date: Some(parse_standard_date("2022-10-20").unwrap()),
            settlement_date: Some(parse_standard_date("2022-10-21").unwrap()),
            action: Some(action),
            shares: Some(dec!(123.1)),
            amount_per_share: Some(dec!(10.1)),
            read_index: 5,
            ..CsvTx::default()
        }
    }

    #[test]
    fn test_csvtx_to_tx_full() {
        // Fully explicit buy
        let buy_csvtx = fully_populated_csvtx(TxAction::Buy);
        let sell_csvtx = fully_populated_csvtx(TxAction::Sell);
        let sfla_csvtx = fully_populated_csvtx(TxAction::Sfla);
        let roc_csvtx = fully_populated_csvtx(TxAction::Roc);

        let exp_buy_tx = Tx{
            security: buy_csvtx.security.clone().unwrap(),
            trade_date: buy_csvtx.trade_date.clone().unwrap(),
            settlement_date: buy_csvtx.settlement_date.clone().unwrap(),
            action_specifics: super::TxActionSpecifics::Buy(BuyTxSpecifics{
                shares: pos(buy_csvtx.shares.unwrap()),
                amount_per_share: gez(buy_csvtx.amount_per_share.unwrap()),
                commission: gez(buy_csvtx.commission.unwrap()),
                tx_currency_and_rate: CurrencyAndExchangeRate::rq_new(
                    buy_csvtx.tx_currency.clone().unwrap(),
                    pos(buy_csvtx.tx_curr_to_local_exchange_rate.unwrap())),
                separate_commission_currency: Some(CurrencyAndExchangeRate::rq_new(
                    buy_csvtx.commission_currency.clone().unwrap(),
                    pos(buy_csvtx.commission_curr_to_local_exchange_rate.unwrap()))),
            }),
            memo: buy_csvtx.memo.clone().unwrap(),
            affiliate: buy_csvtx.affiliate.clone().unwrap(),
            read_index: buy_csvtx.read_index,
        };

        let tx = Tx::try_from(buy_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_buy_tx);

        let mut exp_sell_tx = exp_buy_tx.clone();
        exp_sell_tx.action_specifics = super::TxActionSpecifics::Sell(SellTxSpecifics{
            shares: pos(sell_csvtx.shares.unwrap()),
            amount_per_share: gez(sell_csvtx.amount_per_share.unwrap()),
            commission: gez(sell_csvtx.commission.unwrap()),
            tx_currency_and_rate: CurrencyAndExchangeRate::rq_new(
                sell_csvtx.tx_currency.clone().unwrap(),
                pos(sell_csvtx.tx_curr_to_local_exchange_rate.unwrap())),
            separate_commission_currency: Some(CurrencyAndExchangeRate::rq_new(
                sell_csvtx.commission_currency.clone().unwrap(),
                pos(sell_csvtx.commission_curr_to_local_exchange_rate.unwrap()))),
            specified_superficial_loss: sell_csvtx.specified_superficial_loss.clone(),
        });

        let tx = Tx::try_from(sell_csvtx).unwrap();
        assert_big_struct_eq(tx, exp_sell_tx);

        let mut exp_sfla_tx = exp_buy_tx.clone();
        exp_sfla_tx.action_specifics = super::TxActionSpecifics::Sfla(SflaTxSpecifics{
            total_amount: pos(sfla_csvtx.amount_per_share.unwrap() * sfla_csvtx.shares.unwrap()),
        });

        let tx = Tx::try_from(sfla_csvtx).unwrap();
        assert_big_struct_eq(tx, exp_sfla_tx);

        let mut exp_roc_tx = exp_buy_tx.clone();
        exp_roc_tx.action_specifics = super::TxActionSpecifics::Roc(RocTxSpecifics{
            amount_per_held_share: gez(roc_csvtx.amount_per_share.unwrap()),
            tx_currency_and_rate: CurrencyAndExchangeRate::rq_new(
                roc_csvtx.tx_currency.clone().unwrap(),
                pos(roc_csvtx.tx_curr_to_local_exchange_rate.unwrap())),
        });

        let tx = Tx::try_from(roc_csvtx).unwrap();
        assert_big_struct_eq(tx, exp_roc_tx);
    }

    #[test]
    fn test_csvtx_to_tx_defaults_and_optionals() {
        let buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);

        let exp_buy_tx = Tx{
            security: buy_csvtx.security.clone().unwrap(),
            trade_date: buy_csvtx.trade_date.clone().unwrap(),
            settlement_date: buy_csvtx.settlement_date.clone().unwrap(),
            action_specifics: super::TxActionSpecifics::Buy(BuyTxSpecifics{
                shares: pos(buy_csvtx.shares.unwrap()),
                amount_per_share: gez(buy_csvtx.amount_per_share.unwrap()),
                commission: gez(dec!(0)),
                tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                separate_commission_currency: None,
            }),
            memo: String::new(),
            affiliate: Affiliate::default(),
            read_index: buy_csvtx.read_index,
        };

        let tx = Tx::try_from(buy_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_buy_tx);

        let sell_csvtx = barebones_valid_sample_csvtx(TxAction::Sell);

        let exp_sell_tx = Tx{
            security: sell_csvtx.security.clone().unwrap(),
            trade_date: sell_csvtx.trade_date.clone().unwrap(),
            settlement_date: sell_csvtx.settlement_date.clone().unwrap(),
            action_specifics: super::TxActionSpecifics::Sell(SellTxSpecifics{
                shares: pos(sell_csvtx.shares.unwrap()),
                amount_per_share: gez(sell_csvtx.amount_per_share.unwrap()),
                commission: gez(dec!(0)),
                tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                separate_commission_currency: None,
                specified_superficial_loss: None,
            }),
            memo: String::new(),
            affiliate: Affiliate::default(),
            read_index: sell_csvtx.read_index,
        };

        let tx = Tx::try_from(sell_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_sell_tx);
    }

    #[test]
    fn test_csvtx_to_tx_err() {
        // This could be one of any of the errors.
        let _ = Tx::try_from(CsvTx::default()).unwrap_err();

        // Most errors are going to be pretty straightforward, because
        // of hard type constraints (like PosDecimal, etc) and
        // non-optional types in Tx. There are a lot of such cases,
        // so we'll skip most of those.

        // Check one of the missing optional cases (security)
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.security = None;

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"security\" not found");

        // Interesting Cases:
        // - no action (because it is the only attr we call unwrap() on.
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.action = None;

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"action\" not specified");

        // - currency but no exchange rate (and inverse)
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_currency = Some(Currency::usd());

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"currency\" specified but \"exchange rate\" not found");

        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_curr_to_local_exchange_rate = Some(dec!(1.2));

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"exchange rate\" specified but \"currency\" not found");

        // (double check that CAD doesn't need a rate, and if it does, it must be 1)
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_currency = Some(Currency::cad());

        let tx = Tx::try_from(buy_csvtx).unwrap();
        assert_eq!(tx.buy_specifics().unwrap().tx_currency_and_rate.currency, Currency::cad());
        assert_eq!(tx.buy_specifics().unwrap().tx_currency_and_rate.exchange_rate, pdec!(1));

        // Invalid default rate
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_currency = Some(Currency::cad());
        buy_csvtx.tx_curr_to_local_exchange_rate = Some(dec!(1.2));

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "Default currency (CAD) exchange rate was not 1 (was 1.2)");

        // - comm currency but no rate (and inverse). Just sanity check we're using
        // the same function.
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.commission_currency = Some(Currency::usd());

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"commission currency\" specified but \"commission exchange rate\" not found");
    }
}