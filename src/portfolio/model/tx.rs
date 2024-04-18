use std::fmt::Display;

use rust_decimal::Decimal;
use time::Date;

use crate::portfolio::Affiliate;

use super::currency::{Currency, CurrencyAndExchangeRate};

#[derive(PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
pub struct SFLInput {
	pub superficial_loss: Decimal, // TODO assert this is always negative or zero
	pub force: bool
}

#[derive(PartialEq, Eq, Debug)]
pub struct Tx {
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

impl Tx {
    pub fn tx_currency(&self) -> &Currency {
        &self.tx_currency_and_rate.currency
    }
    pub fn tx_curr_to_local_exchange_rate(&self) -> &Decimal {
        &self.tx_currency_and_rate.exchange_rate
    }

    pub fn commission_currency(&self) -> Currency {
        match &self.separate_commission_currency {
            Some(v) => v.currency.clone(),
            None => self.tx_currency().clone(),
        }
    }

    pub fn commission_curr_to_local_exchange_rate(&self) -> Decimal {
        match &self.separate_commission_currency {
            Some(v) => v.exchange_rate,
            None => self.tx_curr_to_local_exchange_rate().clone(),
        }
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
    use std::{fmt::Debug, iter::zip};
    use std::str::FromStr;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::{Date, Duration, Month};

    use crate::portfolio::{Affiliate, CurrencyAndExchangeRate};

    use super::{Tx, TxAction};

    pub const DEFAULT_SECURITY: &str = "FOO";

    pub fn doy_date(days: i64) -> Date {
        Date::from_calendar_date(2024, Month::January, 1).unwrap()
            .saturating_add(Duration::days(days))
    }

    pub fn tx_default() -> Tx {
        Tx { security: DEFAULT_SECURITY.to_string(),
             trade_date: Date::MIN,
             settlement_date: Date::MIN,
             action: TxAction::Buy,
             shares: dec!(0),
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

    pub fn eprint_vecs_1<T: PartialEq + Debug>(left: Vec<T>, right: Vec<T>) {
        let mut err_str = "left != right. left: [\n".to_string();
        for o in &left {
            err_str += &format!("{:?},\n", o).to_string();
        }
        err_str += "] != right: [\n";
        for o in &right {
            err_str += &format!("{:?},\n", o).to_string();
        }
        eprint!("{}", err_str);
    }

    #[allow(unused)]
    pub fn assert_vec_eq_1<T: PartialEq + Debug>(left: Vec<T>, right: Vec<T>) {
        if left != right {
            eprint_vecs_1(left, right);
            panic!();
        }
    }

    #[allow(unused)]
    pub fn assert_vec_eq_2<T: PartialEq + Debug>(left: Vec<T>, right: Vec<T>) {
        if left != right {
            eprint!("left != right. left: {:#?} !=\n {:#?}", left, right);
            panic!();
        }
    }

    pub fn assert_vec_eq_3<T: PartialEq + Debug>(left: Vec<T>, right: Vec<T>) {
        if left == right {
            return
        }
        if left.len() != right.len() {
            eprint!("size of left ({}) != size of right ({})", left.len(), right.len());
            panic!();
        }
        let mut i = 0;
        for (l, r) in zip(&left, &right) {
            if l != r {
                eprint!("Mismatch at index {}:\n", i);
                eprint!("left: {:#?} != right: {:#?}\n", l, r);
            }

            i += 1;
        }
        panic!();
    }

    use assert_vec_eq_3 as assert_vec_eq;

    #[test]
    #[should_panic]
    #[allow(unused)]
    fn test_decimal_sanity() {
        // Test that Decimal does not allow NaN, and will panic if we
        // try to do create a decimal with NaN.
        let one = Decimal::from_str("1").unwrap();
        let zero = Decimal::from_str("0").unwrap();
        one / zero;
    }

    #[test]
    fn test_tx_order() {
        let mut txs = vec![
            Tx{settlement_date: doy_date(4), read_index: 2, ..tx_default()},
            Tx{settlement_date: doy_date(5), read_index: 1, ..tx_default()},
            Tx{settlement_date: doy_date(2), read_index: 4, ..tx_default()},
            Tx{settlement_date: doy_date(4), read_index: 3, ..tx_default()},
            Tx{settlement_date: doy_date(1), read_index: 5, ..tx_default()},
        ];

        let exp = vec![
            Tx{settlement_date: doy_date(1), read_index: 5, ..tx_default()},
            Tx{settlement_date: doy_date(2), read_index: 4, ..tx_default()},
            Tx{settlement_date: doy_date(4), read_index: 2, ..tx_default()},
            Tx{settlement_date: doy_date(4), read_index: 3, ..tx_default()},
            Tx{settlement_date: doy_date(5), read_index: 1, ..tx_default()},
        ];
        txs.sort();
        assert_vec_eq(txs, exp);
        // assert_eq!(txs, exp);
    }
}