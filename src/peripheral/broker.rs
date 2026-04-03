use crate::portfolio::Affiliate;

use super::sheet_common::SheetParseError;

mod broker_tx;
mod file_detect;
mod fx_tracker;

// Individual brokers
pub mod etrade;
#[cfg(feature = "xlsx_read")]
pub mod questrade;
pub mod rbc_di;

#[derive(Debug)]
pub struct SheetToTxsErr {
    // Note that both of these can be populated at the same time.
    // txs is an incomplete set of the parsed txs.
    pub txs: Option<Vec<BrokerTx>>,
    pub errors: Vec<SheetParseError>,
    pub warnings: Vec<SheetParseError>,
}

pub use broker_tx::*;
pub use file_detect::*;
pub use fx_tracker::*;

pub fn affiliate_for_account_type(account_type: &str) -> Affiliate {
    lazy_static::lazy_static! {
        static ref REGISTERED_RE: regex::Regex =
            regex::RegexBuilder::new(r"rrsp|tfsa|resp|fhsa|rrif")
                .case_insensitive(true)
                .build()
                .unwrap();
    }
    if REGISTERED_RE.is_match(account_type) {
        Affiliate::default_registered()
    } else {
        Affiliate::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affiliate_for_account_type() {
        // Registered accounts
        for acct_type in &[
            "FHSA",
            "fhsa",
            "Individual FHSA",
            "TFSA",
            "RRSP",
            "RESP",
            "RESP Family",
            "RRIF",
        ] {
            assert_eq!(
                affiliate_for_account_type(acct_type),
                Affiliate::default_registered(),
                "Expected registered affiliate for \"{acct_type}\""
            );
        }

        // Non-registered accounts
        for acct_type in &["margin", "Individual margin", "Cash", ""] {
            assert_eq!(
                affiliate_for_account_type(acct_type),
                Affiliate::default(),
                "Expected default affiliate for \"{acct_type}\""
            );
        }
    }
}
