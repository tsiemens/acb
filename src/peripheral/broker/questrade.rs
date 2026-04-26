use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;

use calamine::{Data, Range};
use time::Date;

use crate::{
    peripheral::{
        broker::{Account, BrokerTx, FxTracker, FxtRow},
        sheet_common::SheetParseError,
    },
    portfolio::{Currency, TxAction},
    util::{basic::SError, date::parse_standard_date},
};

use super::SheetToTxsErr;

pub const QUESTRADE_ACCOUNT_BROKER_NAME: &str = "Questrade";

/// Converts a QT spreadsheet into Txs
pub fn sheet_to_txs(
    sheet: &Range<Data>,
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
            ("G036247", ("DLR.TO", "DLR.U.TO")),
        ]);

    // Also, None
    let ignored_actions: HashSet<&'static str> = HashSet::from_iter(
        vec![
            "BRW", "TFI", "TF6", "MGR", "DEP", "NAC", "CON", "INT", "EFT", "RDM",
            "LFJ", "FCH", "",
        ]
        .into_iter(),
    );
    let allowed_actions: HashSet<&'static str> = HashSet::from_iter(
        vec!["BUY", "SELL", "DIS", "LIQ", "FXT", "DIV", "CIL", "REI"].into_iter(),
    );

    // Questrade reports DRIP/REI rows with an internal symbol, so build a
    // description-to-symbol index from ordinary trades in the same export.
    let desc_symbol_aliases = build_desc_symbol_aliases(sheet);

    let mut fx_tracker = FxTracker::new();

    let mut rows = sheet.rows();

    let mut reader =
        crate::peripheral::excel::SheetReader::new(&mut rows).map_err(|e| {
            SheetToTxsErr {
                txs: None,
                errors: vec![e],
                warnings: vec![],
            }
        })?;

    let mut txs = Vec::<BrokerTx>::new();

    let mut errors = Vec::<SheetParseError>::new();
    let mut fatal_error = false;

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

            if action_str == "FXT" {
                let fxt_row = FxtRow {
                    row_num,
                    currency: Currency::new(reader.get_str("Currency")?.as_str()),
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

            // Dividends or Cash-in-lieu should be recorded if foreign currency
            if action_str == "DIV" || action_str == "CIL" {
                if reader.get_str("Currency")?.to_uppercase() == "USD" {
                    let div_tx = FxTracker::fx_tx(
                        Currency::usd(),
                        trade_date,
                        trade_date_str_full.clone(),
                        reader.get_dec("Net Amount")?,
                        row_num,
                        account,
                        None, // exchange rate
                        format!("{action_str} from {pre_alias_symbol}"),
                    )?;
                    fx_tracker.add_income_fx_tx(div_tx);
                }
                return Ok(());
            }

            let description = reader.get_str("Description")?;
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
                "REI" => {
                    // Treat dividend reinvestments as buys using the effective
                    // per-share price implied by the reinvested amount.
                    converted_action_note = "; From REI action.".to_string();
                    TxAction::Buy
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

            let (resolved_symbol, resolved_from_description) = if action_str == "REI"
            {
                let lookup_desc = normalize_symbol_lookup_desc(&description);
                if let Some(symbol) = desc_symbol_aliases.get(&lookup_desc) {
                    (symbol.clone(), true)
                } else if symbol_aliases.contains_key(pre_alias_symbol.as_str()) {
                    // Some Questrade placeholder symbols are already known
                    // manual aliases, so they do not need description lookup.
                    (pre_alias_symbol.clone(), false)
                } else {
                    // Producing partial output here would silently carry the
                    // broker's internal placeholder symbol into the portfolio.
                    fatal_error = true;
                    return Err(err(format!(
                        "Unable to resolve REI symbol {pre_alias_symbol} from \
                         description \"{description}\". Include a matching BUY \
                         or SELL row for this security in the export."
                    )));
                }
            } else {
                (pre_alias_symbol.clone(), false)
            };

            let (symbol, orig_symbol_note) = if let Some((alias, aka)) =
                symbol_aliases.get(resolved_symbol.as_str())
            {
                (alias.to_string(), format!("; {resolved_symbol} AKA {aka}"))
            } else if resolved_from_description {
                (
                    resolved_symbol,
                    format!("; {pre_alias_symbol} resolved from description"),
                )
            } else {
                (resolved_symbol, String::new())
            };

            let num_shares = reader.get_dec("Quantity")?.abs();
            let mut amount_per_share = reader.get_dec("Price")?;
            if action_str == "REI"
                && amount_per_share.is_zero()
                && !num_shares.is_zero()
            {
                amount_per_share = reader.get_dec("Net Amount")?.abs() / num_shares;
            };

            let account_memo = account.memo_str();
            let description_note = if action_str == "REI" {
                format!("; Desc: {description}")
            } else {
                String::new()
            };

            let b_tx = BrokerTx {
                security: symbol,
                trade_date,
                settlement_date,
                trade_date_and_time: trade_date_str_full,
                settlement_date_and_time: settlement_date_str_full,
                action,
                amount_per_share,
                num_shares,
                commission: reader.get_dec("Commission")?.abs(),
                currency: Currency::new(reader.get_str("Currency")?.as_str()),
                memo: account_memo
                    + &orig_symbol_note
                    + &converted_action_note
                    + &description_note,
                exchange_rate: None,
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
            txs: if fatal_error { None } else { Some(txs) },
            errors: errors,
            warnings: vec![],
        })
    } else {
        Ok(txs)
    }
}

fn build_desc_symbol_aliases(sheet: &Range<Data>) -> HashMap<String, String> {
    let mut rows = sheet.rows();
    let mut reader = match crate::peripheral::excel::SheetReader::new(&mut rows) {
        Ok(reader) => reader,
        Err(_) => return HashMap::new(),
    };

    let mut desc_symbol_aliases = HashMap::new();
    for (row_idx, row) in sheet.rows().enumerate().skip(1) {
        reader.set_row(row, row_idx + 1);

        let Ok(action_raw) = reader.get_str("Action") else {
            continue;
        };
        let action = action_raw.to_uppercase();
        if !matches!(action.as_str(), "BUY" | "SELL") {
            // BUY/SELL rows are the only rows that reliably pair Questrade's
            // security description with the public trading symbol. Cash
            // activity can reuse similar descriptions with non-security
            // placeholders, so it must not seed DRIP symbol aliases.
            continue;
        }

        let Ok(symbol) = reader.get_str("Symbol") else {
            continue;
        };
        if symbol.is_empty() {
            continue;
        }

        let Ok(description) = reader.get_str("Description") else {
            continue;
        };
        let key = normalize_symbol_lookup_desc(&description);
        if !key.is_empty() {
            desc_symbol_aliases.entry(key).or_insert(symbol);
        }
    }

    desc_symbol_aliases
}

fn normalize_symbol_lookup_desc(description: &str) -> String {
    // Keep only the security-name prefix. Questrade appends different
    // execution/dividend metadata depending on the row type, while the prefix
    // is the stable part shared by matching BUY/SELL and REI rows.
    const DESC_END_MARKERS: [&str; 4] =
        [" REINV@", " WE ACTED AS ", " REC ", " PAY "];

    let mut end = description.len();
    for marker in DESC_END_MARKERS {
        if let Some(idx) = description.find(marker) {
            end = end.min(idx);
        }
    }
    description[..end].trim().to_string()
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

pub fn extract_questrade_accounts(
    data: &[u8],
    file_name: &str,
    warnings: &mut Vec<String>,
) -> Vec<Account> {
    use crate::peripheral::excel::{read_xl_source, XlSource};

    let rg = match read_xl_source(XlSource::Data(data.to_vec()), None) {
        Ok(rg) => rg,
        Err(e) => {
            warnings.push(format!("{file_name}: {e}"));
            return vec![];
        }
    };
    match sheet_to_txs(&rg, None) {
        Ok(txs) => txs.iter().map(|t| t.account.clone()).collect(),
        Err(e) => {
            // Partial results may still have accounts
            if let Some(ref txs) = e.txs {
                let accounts: Vec<Account> =
                    txs.iter().map(|t| t.account.clone()).collect();
                if !accounts.is_empty() {
                    return accounts;
                }
            }
            for err in &e.errors {
                warnings.push(format!("{file_name}: {err}"));
            }
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripheral::excel::read_xl_data;
    use rust_decimal::Decimal;

    #[cfg(feature = "xlsx_write")]
    fn make_questrade_sheet_bytes(rows: &[Vec<&str>]) -> Vec<u8> {
        use rust_xlsxwriter::Workbook;

        let mut wb = Workbook::new();
        let sheet = wb.add_worksheet();
        let headers = [
            "Transaction Date",
            "Settlement Date",
            "Action",
            "Symbol",
            "Description",
            "Quantity",
            "Price",
            "Gross Amount",
            "Commission",
            "Net Amount",
            "Currency",
            "Account #",
            "Activity Type",
            "Account Type",
        ];

        for (col, value) in headers.iter().enumerate() {
            sheet.write(0, col as u16, *value).unwrap();
        }
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, value) in row.iter().enumerate() {
                sheet.write((row_idx + 1) as u32, col_idx as u16, *value).unwrap();
            }
        }

        wb.save_to_buffer().unwrap()
    }

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_rei_rows_convert_to_buys() {
        let data = make_questrade_sheet_bytes(&[
            vec![
                "2026-02-17 12:00:00 AM",
                "2026-02-19 12:00:00 AM",
                "REI",
                "Z491275",
                "AURORA NORTHERN INCOME ETF SHS REINV@U$42.615 REC 02/11/26 PAY 02/17/26",
                "14.00000",
                "0.00000000",
                "0.00",
                "0.00",
                "-596.61",
                "USD",
                "11112222",
                "Dividend reinvestment",
                "Individual margin",
            ],
            vec![
                "2026-01-09 12:00:00 AM",
                "2026-01-13 12:00:00 AM",
                "Buy",
                "ANIE",
                "AURORA NORTHERN INCOME ETF SHS WE ACTED AS AGENT",
                "9.00000",
                "31.22000000",
                "-280.98",
                "0.00",
                "-280.98",
                "USD",
                "11112222",
                "Trades",
                "Individual margin",
            ],
        ]);

        let range = read_xl_data(data, None).unwrap();
        let txs = sheet_to_txs(&range, None).unwrap();

        let rei_tx = txs.iter().find(|tx| tx.row_num == 2).unwrap();
        assert_eq!(rei_tx.security, "ANIE");
        assert_eq!(rei_tx.action, TxAction::Buy);
        assert_eq!(rei_tx.amount_per_share, Decimal::new(42615, 3));
        assert_eq!(rei_tx.num_shares, Decimal::new(14, 0));
        assert!(rei_tx.memo.contains("resolved from description"));
        assert!(rei_tx.memo.contains("From REI action."));
        assert!(rei_tx.memo.contains(
            "Desc: AURORA NORTHERN INCOME ETF SHS REINV@U$42.615 REC 02/11/26 PAY 02/17/26"
        ));

        let rei_fx = txs
            .iter()
            .find(|tx| tx.security == "USD.FX" && tx.row_num == 2)
            .unwrap();
        assert_eq!(rei_fx.action, TxAction::Sell);
        assert_eq!(rei_fx.num_shares, Decimal::new(59661, 2));
    }

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_rei_without_trade_alias_is_fatal() {
        let data = make_questrade_sheet_bytes(&[
            vec![
                "2026-02-17 12:00:00 AM",
                "2026-02-19 12:00:00 AM",
                "REI",
                "Z491275",
                "AURORA NORTHERN INCOME ETF SHS REINV@U$42.615 REC 02/11/26 PAY 02/17/26",
                "14.00000",
                "0.00000000",
                "0.00",
                "0.00",
                "-596.61",
                "USD",
                "11112222",
                "Dividend reinvestment",
                "Individual margin",
            ],
        ]);

        let range = read_xl_data(data, None).unwrap();
        let err = sheet_to_txs(&range, None).unwrap_err();

        assert!(
            err.txs.is_none(),
            "unresolved REI rows must be fatal so no CSV output is written"
        );
        assert_eq!(err.errors.len(), 1);
        let msg = err.errors[0].to_string();
        assert!(msg.contains("Unable to resolve REI symbol Z491275"));
        assert!(msg.contains("matching BUY or SELL row"));
    }

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_rei_aliases_ignore_non_trade_rows() {
        let data = make_questrade_sheet_bytes(&[
            vec![
                "2026-02-17 12:00:00 AM",
                "2026-02-19 12:00:00 AM",
                "REI",
                "Z491275",
                "AURORA NORTHERN INCOME ETF SHS REINV@U$42.615 REC 02/11/26 PAY 02/17/26",
                "14.00000",
                "0.00000000",
                "0.00",
                "0.00",
                "-596.61",
                "USD",
                "11112222",
                "Dividend reinvestment",
                "Individual margin",
            ],
            vec![
                "2026-02-17 12:00:00 AM",
                "2026-02-17 12:00:00 AM",
                "DIV",
                "ZPOISON",
                "AURORA NORTHERN INCOME ETF SHS REC 02/11/26 PAY 02/17/26",
                "0.00000",
                "0.00000000",
                "0.00",
                "0.00",
                "10.00",
                "USD",
                "11112222",
                "Dividends",
                "Individual margin",
            ],
            vec![
                "2026-01-09 12:00:00 AM",
                "2026-01-13 12:00:00 AM",
                "Buy",
                "ANIE",
                "AURORA NORTHERN INCOME ETF SHS WE ACTED AS AGENT",
                "9.00000",
                "31.22000000",
                "-280.98",
                "0.00",
                "-280.98",
                "USD",
                "11112222",
                "Trades",
                "Individual margin",
            ],
        ]);

        let range = read_xl_data(data, None).unwrap();
        let txs = sheet_to_txs(&range, None).unwrap();

        let rei_tx = txs.iter().find(|tx| tx.row_num == 2).unwrap();
        assert_eq!(rei_tx.security, "ANIE");
        assert!(rei_tx.memo.contains("Z491275 resolved from description"));
        assert!(
            !txs.iter().any(|tx| tx.security == "ZPOISON"),
            "non-trade rows must not be used as REI symbol aliases"
        );
    }
}
