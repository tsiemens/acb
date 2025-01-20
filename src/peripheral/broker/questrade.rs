use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;

use office::Range;
use time::Date;

use crate::{
    peripheral::{
        broker::{Account, BrokerTx, FxTracker, FxtRow},
        sheet_common::SheetParseError,
    },
    portfolio::{Affiliate, Currency, TxAction},
    util::{basic::SError, date::parse_standard_date},
};

use super::SheetToTxsErr;

const QUESTRADE_ACCOUNT_BROKER_NAME: &str = "Questrade";

/// Converts a QT spreadsheet into Txs
pub fn sheet_to_txs(
    sheet: &Range,
    fpath: Option<&std::path::Path>,
) -> Result<Vec<BrokerTx>, SheetToTxsErr> {
    // Column names:
    //  'Transaction Date', 'Settlement Date', 'Action''Symbol', 'Description',
    //  'Quantity', 'Price', 'Gross Amount', 'Commission', 'Net Amount',
    //  'Currency', 'Account #', 'Activity Type', 'Account Type'

    let symbol_aliases =
        HashMap::<&'static str, (&'static str, &'static str)>::from([
            // symbol : (alias_to, AKA)
            ("H038778", ("DLR.TO", "DLR.U.TO")),
        ]);

    // Also, None
    let ignored_actions: HashSet<&'static str> = HashSet::from_iter(
        vec![
            "BRW", "TFI", "TF6", "MGR", "DEP", "NAC", "CON", "INT", "EFT", "RDM", "",
        ]
        .into_iter(),
    );
    let allowed_actions: HashSet<&'static str> = HashSet::from_iter(
        vec!["BUY", "SELL", "DIS", "LIQ", "FXT", "DIV"].into_iter(),
    );

    let mut fx_tracker = FxTracker::new();

    let mut rows = sheet.rows();

    let mut reader =
        crate::peripheral::excel::SheetReader::new(&mut rows).map_err(|e| {
            SheetToTxsErr {
                txs: None,
                errors: vec![e],
            }
        })?;

    let mut txs = Vec::<BrokerTx>::new();

    let mut errors = Vec::<SheetParseError>::new();

    let mut row_num = 0;
    for row in sheet.rows() {
        let row_res = (|| {
            // Header counts as first row, so we start at 2.
            row_num += 1;
            if row_num == 1 {
                // Skip header
                return Ok(());
            }

            reader.set_row(row, row_num);

            let err = |s| SheetParseError::new(row_num, s);

            let action_str_raw = reader.get_str("Action")?;
            let action_str: String = action_str_raw.to_uppercase();
            if !allowed_actions.contains(action_str.as_str())
                && !ignored_actions.contains(action_str.as_str())
            {
                return Err(err(format!(
                    "Unrecognized transaction action {action_str_raw}"
                )));
            }
            if ignored_actions.contains(action_str.as_str()) {
                return Ok(());
            }

            let trade_date_str_full = reader.get_str("Transaction Date")?;
            let trade_date = convert_date_str(&trade_date_str_full).map_err(err)?;
            let settlement_date_str_full = reader.get_str("Settlement Date")?;
            let settlement_date =
                convert_date_str(&settlement_date_str_full).map_err(err)?;

            let account_type = reader.get_str("Account Type")?;
            let account_num = reader.get_str("Account #")?;
            let account = Account {
                broker_name: QUESTRADE_ACCOUNT_BROKER_NAME,
                account_type,
                account_num,
            };

            let affiliate = if regex::RegexBuilder::new(r"rrsp|tfsa|resp")
                .case_insensitive(true)
                .build()
                .unwrap()
                .is_match(&account.account_type)
            {
                Affiliate::default_registered()
            } else {
                Affiliate::default()
            };

            if action_str == "FXT" {
                let fxt_row = FxtRow {
                    row_num,
                    currency: Currency::new(reader.get_str("Currency")?.as_str()),
                    affiliate,
                    trade_date,
                    trade_date_and_time: trade_date_str_full.clone(),
                    amount: reader.get_dec("Net Amount")?,
                    account,
                };
                fx_tracker.add_fxt_row(fxt_row)?;
                return Ok(());
            }

            let pre_alias_symbol = reader.get_str("Symbol")?;
            if pre_alias_symbol.is_empty() {
                return Err(SheetParseError::new(
                    row_num,
                    "Symbol was empty".to_string(),
                ));
            }

            if action_str == "DIV" {
                if reader.get_str("Currency")?.to_uppercase() == "USD" {
                    let div_tx = FxTracker::fx_tx(
                        Currency::usd(),
                        trade_date,
                        trade_date_str_full.clone(),
                        reader.get_dec("Net Amount")?,
                        affiliate,
                        row_num,
                        account,
                        None, // exchange rate
                        format!("DIV from {pre_alias_symbol}"),
                    )?;
                    fx_tracker.add_income_fx_tx(div_tx);
                }
                return Ok(());
            }

            let mut converted_action_note = String::new();
            let action = match action_str.as_str() {
                "BUY" => TxAction::Buy,
                "SELL" => TxAction::Sell,
                "DIS" => {
                    // Treat stock distributions as free purchases
                    // (the amount will be zero).
                    converted_action_note = "; From DIS action.".to_string();
                    TxAction::Buy
                }
                "LIQ" => {
                    // Treat stock liquidations as sales
                    converted_action_note = "; From LIQ action.".to_string();
                    TxAction::Sell
                }
                _ => {
                    // This should never happen in practice, if we handle all
                    // allowed actions above.
                    return Err(err(format!(
                        "Internal error: Unhandled type {} ",
                        action_str
                    )));
                }
            };

            let (symbol, orig_symbol_note) = if let Some((alias, aka)) =
                symbol_aliases.get(pre_alias_symbol.as_str())
            {
                (alias.to_string(), format!("; {pre_alias_symbol} AKA {aka}"))
            } else {
                (pre_alias_symbol, String::new())
            };

            let account_memo = account.memo_str();

            let b_tx = BrokerTx {
                security: symbol,
                trade_date,
                settlement_date,
                trade_date_and_time: trade_date_str_full,
                settlement_date_and_time: settlement_date_str_full,
                action,
                amount_per_share: reader.get_dec("Price")?,
                num_shares: reader.get_dec("Quantity")?.abs(),
                commission: reader.get_dec("Commission")?.abs(),
                currency: Currency::new(reader.get_str("Currency")?.as_str()),
                memo: account_memo + &orig_symbol_note + &converted_action_note,
                exchange_rate: None,
                affiliate,
                row_num: row_num as u32,
                account: account,
                sort_tiebreak: None,
                filename: fpath.map(|p| p.to_string_lossy().to_string()),
            };
            txs.push(b_tx.clone());

            if !b_tx.currency.is_default() {
                fx_tracker.add_implicit_fxt(&b_tx)?;
            }
            Ok(())
        })();

        if let Err(e) = row_res {
            errors.push(e)
        }
    } // END for row in rows

    // Add the FXTs
    let mut fx_txs: Vec<BrokerTx> = match fx_tracker.get_fx_txs() {
        Ok(txs) => txs,
        Err((txs, e)) => {
            errors.push(e);
            txs
        }
    }
    .iter()
    .map(|t| (*t).clone())
    .collect();
    // These will get sorted by the caller.
    txs.append(&mut fx_txs);

    if errors.len() > 0 {
        Err(SheetToTxsErr {
            txs: Some(txs),
            errors: errors,
        })
    } else {
        Ok(txs)
    }
}

lazy_static! {
    static ref DATE_REGEXP: regex::Regex =
        regex::Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap();
}

/// Converts the date format present in Qt sheets
fn convert_date_str(date_str: &str) -> Result<Date, SError> {
    let m = DATE_REGEXP
        .find(date_str)
        .ok_or(format!("Unable to parse date \"{date_str}\""))?;
    parse_standard_date(m.as_str()).map_err(|e| format!("{e}"))
}
