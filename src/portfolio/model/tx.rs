use std::fmt::Display;

use rust_decimal::Decimal;
use time::Date;

use crate::{portfolio::Affiliate, util::decimal::{GreaterEqualZeroDecimal, LessEqualZeroDecimal, PosDecimal}};

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

// This Transaction type is flat, and designed to absorb input from a uniform
// CSV/table input. It is fairly unconstrained, and as such, should be converted
// to the Tx type for processing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CsvTx {
    pub security: String,
    pub trade_date: Date,
    pub settlement_date: Date,
    pub action: TxAction,
    pub shares: Decimal,
    pub amount_per_share: Decimal,
    pub commission: Decimal,

    pub tx_currency_and_rate: CurrencyAndExchangeRate,
    pub separate_commission_currency: Option<CurrencyAndExchangeRate>,

    pub memo: String,
    pub affiliate: Affiliate,

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

impl CsvTx {
    pub fn tx_currency(&self) -> &Currency {
        &self.tx_currency_and_rate.currency
    }
    pub fn tx_curr_to_local_exchange_rate(&self) -> &PosDecimal {
        &self.tx_currency_and_rate.exchange_rate
    }

    pub fn commission_currency(&self) -> Currency {
        match &self.separate_commission_currency {
            Some(v) => v.currency.clone(),
            None => self.tx_currency().clone(),
        }
    }

    pub fn commission_curr_to_local_exchange_rate(&self) -> PosDecimal {
        match &self.separate_commission_currency {
            Some(v) => v.exchange_rate,
            None => self.tx_curr_to_local_exchange_rate().clone(),
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

// For now, just return BuyTxSpecifics, since it is a strict subset of
// SellTxSpecifics
fn buy_or_sell_common_attrs_from_csv_tx(csv_tx: &CsvTx, buy_or_sell: &str) -> Result<BuyTxSpecifics, String> {
    let specifics = BuyTxSpecifics{
        shares: PosDecimal::try_from(csv_tx.shares).map_err(|_| format!(
            "{buy_or_sell} shares must be a positive value"))?,
        amount_per_share: GreaterEqualZeroDecimal::try_from(csv_tx.shares).map_err(|_| format!(
            "{buy_or_sell} amount/share must not be negative"))?,
        commission: GreaterEqualZeroDecimal::try_from(csv_tx.shares).map_err(|_| format!(
            "{buy_or_sell} comission must not be negative"))?,
        tx_currency_and_rate: csv_tx.tx_currency_and_rate.clone(),
        separate_commission_currency: csv_tx.separate_commission_currency.clone(),
    };
    Ok(specifics)
}

impl TryFrom<CsvTx> for Tx {
    type Error = String;

    fn try_from(csv_tx: CsvTx) -> Result<Self, Self::Error> {
        let act_specs = match csv_tx.action {
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
                let amount = GreaterEqualZeroDecimal::try_from(
                    csv_tx.amount_per_share).map_err(|_| format!(
                        "RoC amount per share must not be negative. Found {}", csv_tx.amount_per_share))?;
                TxActionSpecifics::Roc(RocTxSpecifics{
                    amount_per_held_share: amount,
                    tx_currency_and_rate: csv_tx.tx_currency_and_rate,
                })
            },
            TxAction::Sfla => {
                let total = PosDecimal::try_from(
                    csv_tx.amount_per_share * csv_tx.shares).map_err(
                        |_| format!(
                            "SfLA (amount per share) x (shares) must be positive ({} x {})",
                            csv_tx.amount_per_share, csv_tx.shares))?;
                TxActionSpecifics::Sfla(SflaTxSpecifics{
                    total_amount: total,
                })
            },
        };

        let tx = Tx{
            security: csv_tx.security,
            trade_date: csv_tx.trade_date,
            settlement_date: csv_tx.settlement_date,
            action_specifics: act_specs,
            memo: csv_tx.memo,
            affiliate: csv_tx.affiliate,
            read_index: csv_tx.read_index,
        };
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
    use rust_decimal_macros::dec;
    use time::Date;

    use crate::portfolio::{Affiliate, CurrencyAndExchangeRate};
    use crate::testlib::assert_vec_eq;
    use crate::util::date;

    use super::{CsvTx, Tx, TxAction};

    pub const DEFAULT_SECURITY: &str = "FOO";

    pub fn tx_default() -> CsvTx {
        CsvTx { security: DEFAULT_SECURITY.to_string(),
             trade_date: Date::MIN,
             settlement_date: Date::MIN,
             action: TxAction::Buy,
             shares: dec!(1),
             amount_per_share: dec!(0),
             commission: dec!(0),
             tx_currency_and_rate: CurrencyAndExchangeRate::default(),
             separate_commission_currency: None,
             memo: "".to_string(),
             affiliate: Affiliate::default(),
             specified_superficial_loss: None,
             read_index: 0,
            }
    }

    #[test]
    fn test_tx_order() {
        let doy_date = |day| { date::pub_testlib::doy_date(2024, day) };

        let mut csv_txs = vec![
            CsvTx{settlement_date: doy_date(4), read_index: 2, ..tx_default()},
            CsvTx{settlement_date: doy_date(5), read_index: 1, ..tx_default()},
            CsvTx{settlement_date: doy_date(2), read_index: 4, ..tx_default()},
            CsvTx{settlement_date: doy_date(4), read_index: 3, ..tx_default()},
            CsvTx{settlement_date: doy_date(1), read_index: 5, ..tx_default()},
        ];
        let mut txs: Vec<Tx> = csv_txs.iter().map(|c_tx| {
            Tx::try_from(c_tx.clone()).unwrap() }).collect();

        let exp_csv_txs = vec![
            CsvTx{settlement_date: doy_date(1), read_index: 5, ..tx_default()},
            CsvTx{settlement_date: doy_date(2), read_index: 4, ..tx_default()},
            CsvTx{settlement_date: doy_date(4), read_index: 2, ..tx_default()},
            CsvTx{settlement_date: doy_date(4), read_index: 3, ..tx_default()},
            CsvTx{settlement_date: doy_date(5), read_index: 1, ..tx_default()},
        ];
        let exp_txs: Vec<Tx> = exp_csv_txs.iter().map(|c_tx| {
            Tx::try_from(c_tx.clone()).unwrap() }).collect();

        csv_txs.sort();
        assert_vec_eq(csv_txs, exp_csv_txs);

        // Tx should have the same order
        txs.sort();
        assert_vec_eq(txs, exp_txs);
    }
}