use std::collections::HashSet;

use crate::app::config::AcbConfig;
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

pub fn is_registered_account_type(account_type: &str) -> bool {
    lazy_static::lazy_static! {
        static ref REGISTERED_RE: regex::Regex =
            regex::RegexBuilder::new(r"rrsp|tfsa|resp|fhsa|rrif")
                .case_insensitive(true)
                .build()
                .unwrap();
    }
    REGISTERED_RE.is_match(account_type)
}

/// Map a `BrokerTx::account.broker_name` to the config key used in
/// `AcbConfig.account_bindings`.
pub fn config_key_for_broker_name(broker_name: &str) -> Option<&'static str> {
    use crate::app::config;
    match broker_name {
        etrade::ETRADE_ACCOUNT_BROKER_NAME => Some(config::ETRADE_CONFIG_KEY),
        rbc_di::RBC_DI_BROKER_NAME => Some(config::RBC_DI_CONFIG_KEY),
        #[cfg(feature = "xlsx_read")]
        questrade::QUESTRADE_ACCOUNT_BROKER_NAME => {
            Some(config::QUESTRADE_CONFIG_KEY)
        }
        _ => None,
    }
}

/// Determine the affiliate for a broker account, consulting the config first.
///
/// If the config contains a binding for this broker + account number, the
/// affiliate name comes from the config and the registered flag is derived
/// from the broker-detected account type.
///
/// If no config match is found, returns Default or Default (R) based on
/// account type.
pub fn affiliate_for_account_with_config(
    account: &Account,
    config: Option<&AcbConfig>,
) -> Affiliate {
    let registered = is_registered_account_type(&account.account_type);
    if let Some(cfg) = config {
        if let Some(config_key) = config_key_for_broker_name(account.broker_name) {
            if let Some(af_name) =
                cfg.affiliate_name_for_account(config_key, &account.account_num)
            {
                return Affiliate::from_base_name(af_name, registered);
            }
        }
    }
    if registered {
        Affiliate::default_registered()
    } else {
        Affiliate::default()
    }
}

/// Extract unique accounts from broker file data.
///
/// For each file, the kind is detected and the appropriate broker parser
/// runs to extract `Account` structs. Results are deduplicated.
///
/// `files` is a slice of (raw_data, file_name) pairs.
/// `pdf_page_texts` provides pre-parsed PDF page texts: a slice of
/// (pages, file_name) pairs, one per PDF file.
///
/// Returns `(accounts, warnings)`.
#[cfg(feature = "xlsx_read")]
pub fn extract_accounts_from_files(
    files: &[(&[u8], &str)],
    pdf_page_texts: &[(&[String], &str)],
) -> (Vec<Account>, Vec<String>) {
    let mut seen = HashSet::<Account>::new();
    let mut warnings = Vec::new();

    // Process non-PDF files (xlsx, csv)
    for &(data, file_name) in files {
        let detect_result =
            match detect_file_kind(FileDetectSource::Bytes { data, file_name }) {
                Ok(r) => r,
                Err(e) => {
                    warnings.push(format!("{file_name}: {e}"));
                    continue;
                }
            };

        let accounts: Vec<Account> = match detect_result.kind {
            FileKind::QuestradeExcel => {
                use crate::peripheral::broker::questrade::extract_questrade_accounts;

                extract_questrade_accounts(data, file_name, &mut warnings)
            }
            FileKind::RbcDiCsv => {
                use crate::peripheral::broker::rbc_di::extract_rbc_di_accounts;

                extract_rbc_di_accounts(data, file_name, &mut warnings)
            }
            FileKind::EtradeBenefitsExcel => {
                // Etrade xlsx files don't contain account numbers
                continue;
            }
            _ => continue,
        };

        for account in accounts {
            seen.insert(account);
        }
    }

    // Process PDF files (already have page texts from JS-side parsing)
    for &(ref pages, file_name) in pdf_page_texts {
        let detect_result = detect_file_kind(FileDetectSource::PdfPages(pages))
            .unwrap_or(FileDetectResult {
                kind: FileKind::Unknown,
                warning: None,
            });

        match detect_result.kind {
            FileKind::EtradeTradeConfirmationPdf | FileKind::EtradeBenefitPdf => {
                use crate::peripheral::broker::etrade::extract_etrade_pdf_accounts;

                let full_text = pages.join("\n");
                let accounts = extract_etrade_pdf_accounts(
                    &full_text,
                    file_name,
                    &mut warnings,
                );
                for account in accounts {
                    seen.insert(account);
                }
            }
            _ => continue,
        }
    }

    let accounts: Vec<Account> = seen.into_iter().collect();
    (accounts, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_registered_account_type() {
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
            assert!(
                is_registered_account_type(acct_type),
                "Expected registered for \"{acct_type}\""
            );
        }

        for acct_type in &["margin", "Individual margin", "Cash", ""] {
            assert!(
                !is_registered_account_type(acct_type),
                "Expected non-registered for \"{acct_type}\""
            );
        }
    }

    fn make_account(
        broker_name: &'static str,
        account_type: &str,
        account_num: &str,
    ) -> Account {
        Account {
            broker_name,
            account_type: account_type.to_string(),
            account_num: account_num.to_string(),
        }
    }

    #[test]
    fn test_affiliate_for_account_with_config_no_config() {
        let account = make_account("Questrade", "TFSA", "12345");
        let af = affiliate_for_account_with_config(&account, None);
        assert_eq!(af, Affiliate::default_registered());
    }

    #[test]
    fn test_affiliate_for_account_with_config_no_match() {
        let config = crate::app::config::AcbConfig::new();
        let account = make_account("Questrade", "TFSA", "12345");
        let af = affiliate_for_account_with_config(&account, Some(&config));
        assert_eq!(af, Affiliate::default_registered());
    }

    #[test]
    fn test_affiliate_for_account_with_config_match_registered() {
        let mut config = crate::app::config::AcbConfig::new();
        config
            .account_bindings
            .questrade
            .insert("12345".to_string(), "Spouse".to_string());

        let account = make_account("Questrade", "TFSA", "12345");
        let af = affiliate_for_account_with_config(&account, Some(&config));
        // Config name "Spouse" + registered from account type
        assert_eq!(af, Affiliate::from_strep("Spouse (R)"));
        assert!(af.registered());
    }

    #[test]
    fn test_affiliate_for_account_with_config_match_non_registered() {
        let mut config = crate::app::config::AcbConfig::new();
        config
            .account_bindings
            .questrade
            .insert("12345".to_string(), "Spouse".to_string());

        let account = make_account("Questrade", "Margin", "12345");
        let af = affiliate_for_account_with_config(&account, Some(&config));
        assert_eq!(af, Affiliate::from_strep("Spouse"));
        assert!(!af.registered());
    }

    #[test]
    fn test_affiliate_for_account_with_config_etrade() {
        let mut config = crate::app::config::AcbConfig::new();
        config
            .account_bindings
            .etrade
            .insert("XXXX-1234".to_string(), "Spouse".to_string());

        let account = make_account("E*TRADE", "Brokerage", "XXXX-1234");
        let af = affiliate_for_account_with_config(&account, Some(&config));
        assert_eq!(af, Affiliate::from_strep("Spouse"));
        assert!(!af.registered());
    }

    #[test]
    fn test_affiliate_for_account_with_config_rbc_di() {
        let mut config = crate::app::config::AcbConfig::new();
        config
            .account_bindings
            .rbc_di
            .insert("99999".to_string(), "Spouse".to_string());

        let account = make_account("RBC Direct Investing", "RRSP", "99999");
        let af = affiliate_for_account_with_config(&account, Some(&config));
        assert_eq!(af, Affiliate::from_strep("Spouse (R)"));
    }
}
