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

// TODO TxDelta (put in txdelta.rs)

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use rust_decimal::Decimal;

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
}