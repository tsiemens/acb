use rust_decimal::Decimal;
use time::Date;

use crate::portfolio::{Affiliate, Currency, Security, TxAction};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Account {
    pub broker_name: &'static str,
    pub account_type: String,
    // This is a string rather than a number because it might
    // have hyphens or letters in it
    pub account_num: String,
}

impl Account {
    pub fn account_str(&self) -> String {
        format!("{} {}", self.account_type, self.account_num)
    }

    pub fn memo_str(&self) -> String {
        format!("{} {}", self.broker_name, self.account_str())
    }
}

/// This is similar to CsvTx, but slightly less constrained, as it
/// represents a Tx imported from any broker's exported data.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct BrokerTx {
    pub security: Security,
    pub trade_date: Date,
    pub settlement_date: Date,
    pub trade_date_and_time: String, // Just used for sort tiebreaking
    pub settlement_date_and_time: String, // Just used for sort tiebreaking
    pub action: TxAction,
    pub amount_per_share: Decimal,
    pub num_shares: Decimal,
    pub commission: Decimal,
    pub currency: Currency,
    pub memo: String,
    pub exchange_rate: Option<Decimal>,
    pub affiliate: Affiliate,

    pub row_num: u32,
    pub account: Account,
    /// Only has an effect if two Txs both have non-None values
    pub sort_tiebreak: Option<u32>,

    pub filename: Option<String>,
}

impl PartialOrd for BrokerTx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let sdate_cmp = self.settlement_date.cmp(&other.settlement_date);
        match sdate_cmp {
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater =>
                return Some(sdate_cmp),
            std::cmp::Ordering::Equal => (),
        }
        let sdate_and_time_cmp = self.settlement_date_and_time.cmp(&other.settlement_date_and_time);
        match sdate_and_time_cmp {
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater =>
                return Some(sdate_and_time_cmp),
            std::cmp::Ordering::Equal => (),
        }
        if let Some(st) = self.sort_tiebreak {
            if let Some(ost) = &other.sort_tiebreak {
                let sort_tiebreak_cmp = st.cmp(ost);
                match sort_tiebreak_cmp {
                    std::cmp::Ordering::Less | std::cmp::Ordering::Greater =>
                        return Some(sort_tiebreak_cmp),
                    std::cmp::Ordering::Equal => (),
                }
            } else {
                return Some(std::cmp::Ordering::Greater);
            }
        } else {
            if other.sort_tiebreak.is_some() {
                return Some(std::cmp::Ordering::Less);
            }
        }
        // Neither have sort tiebreak

        Some(self.row_num.cmp(&other.row_num))
    }
}

impl Ord for BrokerTx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Into<crate::portfolio::CsvTx> for BrokerTx {
    fn into(self) -> crate::portfolio::CsvTx {
        crate::portfolio::CsvTx {
            security: Some(self.security),
            trade_date: Some(self.trade_date),
            settlement_date: Some(self.settlement_date),
            action: Some(self.action),
            shares: Some(self.num_shares),
            amount_per_share: Some(self.amount_per_share),
            commission: Some(self.commission),
            tx_currency: Some(self.currency),
            tx_curr_to_local_exchange_rate: self.exchange_rate,
            commission_currency: None,
            commission_curr_to_local_exchange_rate: None,
            memo: Some(self.memo),
            affiliate: Some(self.affiliate),
            specified_superficial_loss: None,
            read_index: self.row_num,
        }
    }
}