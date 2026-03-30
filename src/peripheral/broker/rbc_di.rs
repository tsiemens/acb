use std::collections::HashSet;

use rust_decimal::Decimal;
use time::Date;

use crate::{
    peripheral::{
        broker::{Account, BrokerTx},
        sheet_common::SheetParseError,
    },
    portfolio::{Affiliate, Currency, TxAction},
    util::basic::SError,
};

use super::SheetToTxsErr;

const RBC_DI_BROKER_NAME: &str = "RBC Direct Investing";

// Expected header columns
const COL_DATE: &str = "Date";
const COL_ACTIVITY: &str = "Activity";
const COL_SYMBOL: &str = "Symbol";
const COL_QUANTITY: &str = "Quantity";
const COL_PRICE: &str = "Price";
const COL_SETTLEMENT_DATE: &str = "Settlement Date";
const COL_ACCOUNT: &str = "Account";
const COL_VALUE: &str = "Value";
const COL_CURRENCY: &str = "Currency";

/// Columns that must be present in the header row for it to be recognized.
pub(crate) const REQUIRED_HEADERS: &[&str] = &[
    COL_DATE,
    COL_ACTIVITY,
    COL_SYMBOL,
    COL_QUANTITY,
    COL_PRICE,
    COL_SETTLEMENT_DATE,
    COL_ACCOUNT,
    COL_VALUE,
    COL_CURRENCY,
];

/// Activity types that produce BrokerTx (buy/sell).
const TRADE_ACTIVITIES: &[&str] = &["buy", "sell"];

/// Activity types that are silently skipped (no warning).
const SILENT_SKIP_ACTIVITIES: &[&str] =
    &["distribution", "deposits & contributions", "fees", "dividends"];

/// Activity types that are skipped with a warning.
const WARN_SKIP_ACTIVITIES: &[&str] = &["reorganization", "return of capital"];

/// RBC DI date format: "December 24, 2025"
const RBC_DATE_FORMAT: &[time::format_description::BorrowedFormatItem<'_>] = time::macros::format_description!(
    "[month repr:long] [day padding:none], [year]"
);

fn parse_rbc_date(date_str: &str) -> Result<Date, SError> {
    Date::parse(date_str.trim(), RBC_DATE_FORMAT)
        .map_err(|e| format!("Unable to parse date \"{date_str}\": {e}"))
}

fn parse_decimal(
    val: &str,
    col_name: &str,
    row_num: usize,
) -> Result<Decimal, SheetParseError> {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return Err(SheetParseError::new(
            row_num,
            format!("value in {col_name} was empty"),
        ));
    }
    trimmed.parse::<Decimal>().map_err(|e| {
        SheetParseError::new(
            row_num,
            format!("Unable to parse number from \"{trimmed}\" in {col_name}: {e}"),
        )
    })
}

/// Detect the account type from the preamble line like
/// "Account: 12345678 - RRSP|RESP <type>|TFSA|..."
fn parse_preamble_account_info(
    preamble_lines: &[String],
) -> Option<(String, String)> {
    lazy_static::lazy_static! {
        static ref ACCOUNT_RE: regex::Regex =
            regex::Regex::new(r#"^"?Account:\s*(\d+)\s*-\s*(.+?)"?$"#).unwrap();
    }
    for line in preamble_lines {
        if let Some(caps) = ACCOUNT_RE.captures(line.trim()) {
            let account_num = caps[1].to_string();
            let account_type = caps[2].trim().trim_matches('"').to_string();
            return Some((account_num, account_type));
        }
    }
    None
}

fn affiliate_for_account_type(account_type: &str) -> Affiliate {
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

/// A simple column-index lookup for CSV header rows.
struct CsvColMap {
    col_to_index: std::collections::HashMap<String, usize>,
}

impl CsvColMap {
    fn from_record(record: &csv::StringRecord) -> Self {
        let col_to_index = record
            .iter()
            .enumerate()
            .map(|(i, col)| (col.trim().to_lowercase(), i))
            .collect();
        CsvColMap { col_to_index }
    }

    fn get_str<'a>(
        &self,
        record: &'a csv::StringRecord,
        col_name: &str,
    ) -> Result<&'a str, String> {
        let idx = self
            .col_to_index
            .get(&col_name.to_lowercase())
            .ok_or_else(|| format!("Column \"{col_name}\" not found"))?;
        Ok(record.get(*idx).unwrap_or("").trim())
    }
}

/// Scans `csv_data` for the RBC DI header row.
///
/// Returns `(preamble_lines, header_line_index, csv_bytes_from_header)` where
/// `csv_bytes_from_header` is the portion of the input starting at the header row,
/// suitable for feeding into a csv::Reader.
fn find_header_and_split(csv_data: &[u8]) -> Result<(Vec<String>, usize), SError> {
    let text = std::str::from_utf8(csv_data)
        .map_err(|e| format!("CSV data is not valid UTF-8: {e}"))?;

    let required: HashSet<String> =
        REQUIRED_HEADERS.iter().map(|h| h.to_lowercase()).collect();

    let mut preamble = Vec::new();
    for (i, line) in text.lines().enumerate() {
        // Parse this line as CSV to handle quoted fields
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());
        if let Some(Ok(record)) = rdr.records().next() {
            let cols: HashSet<String> =
                record.iter().map(|c| c.trim().to_lowercase()).collect();
            if required.iter().all(|h| cols.contains(h)) {
                return Ok((preamble, i));
            }
        }
        preamble.push(line.to_string());
    }

    Err("Could not find RBC DI CSV header row. Expected columns: Date, Activity, Symbol, Quantity, Price, Settlement Date, Account, Value, Currency".to_string())
}

/// Converts an RBC Direct Investing CSV export into BrokerTx transactions.
pub fn csv_to_txs(
    csv_data: &[u8],
    fpath: Option<&std::path::Path>,
) -> Result<Vec<BrokerTx>, SheetToTxsErr> {
    let text = std::str::from_utf8(csv_data).map_err(|e| SheetToTxsErr {
        txs: None,
        errors: vec![SheetParseError::new(
            0,
            format!("CSV is not valid UTF-8: {e}"),
        )],
        warnings: vec![],
    })?;

    let (preamble, header_line_idx) =
        find_header_and_split(csv_data).map_err(|e| SheetToTxsErr {
            txs: None,
            errors: vec![SheetParseError::new(0, e)],
            warnings: vec![],
        })?;

    let (preamble_account_num, preamble_account_type) =
        parse_preamble_account_info(&preamble).unwrap_or_default();

    let affiliate = affiliate_for_account_type(&preamble_account_type);

    // Build a csv reader from the header line onward
    let csv_portion: String =
        text.lines().skip(header_line_idx).collect::<Vec<&str>>().join("\n");

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_portion.as_bytes());

    let headers = rdr
        .headers()
        .map_err(|e| SheetToTxsErr {
            txs: None,
            errors: vec![SheetParseError::new(
                header_line_idx + 1,
                format!("Failed to parse CSV headers: {e}"),
            )],
            warnings: vec![],
        })?
        .clone();

    let col_map = CsvColMap::from_record(&headers);

    let silent_skip: HashSet<&str> =
        HashSet::from_iter(SILENT_SKIP_ACTIVITIES.iter().copied());
    let warn_skip: HashSet<&str> =
        HashSet::from_iter(WARN_SKIP_ACTIVITIES.iter().copied());
    let trade_activities: HashSet<&str> =
        HashSet::from_iter(TRADE_ACTIVITIES.iter().copied());

    let mut txs = Vec::<BrokerTx>::new();
    let mut errors = Vec::<SheetParseError>::new();
    let mut warnings = Vec::<SheetParseError>::new();

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue, // Skip unparseable rows (footer, etc.)
        };

        // Row number in the original file (1-indexed, accounting for preamble + header)
        let row_num =
            header_line_idx + 1 + record.position().map_or(0, |p| p.line() as usize);

        enum RowResult {
            Ok,
            Warning(SheetParseError),
            Err(SheetParseError),
        }

        let row_res: RowResult = (|| {
            let err = |s: String| SheetParseError::new(row_num, s);

            let activity_raw =
                col_map.get_str(&record, COL_ACTIVITY).map_err(|e| err(e))?;
            let activity_lower = activity_raw.to_lowercase();

            // Check if this is a footer/disclaimer line
            if activity_raw.is_empty() {
                return Result::<RowResult, SheetParseError>::Ok(RowResult::Ok);
            }

            if silent_skip.contains(activity_lower.as_str()) {
                return Ok(RowResult::Ok);
            }

            if warn_skip.contains(activity_lower.as_str()) {
                return Ok(RowResult::Warning(err(format!(
                    "\"{activity_raw}\" transactions are not automatically converted. \
                     Please add this transaction manually."
                ))));
            }

            if !trade_activities.contains(activity_lower.as_str()) {
                return Err(err(format!(
                    "Unrecognized activity type \"{activity_raw}\""
                )));
            }

            let symbol = col_map.get_str(&record, COL_SYMBOL).map_err(|e| err(e))?;
            if symbol.is_empty() {
                return Err(err("Symbol was empty".to_string()));
            }

            let action = match activity_lower.as_str() {
                "buy" => TxAction::Buy,
                "sell" => TxAction::Sell,
                _ => unreachable!(),
            };

            let trade_date_str =
                col_map.get_str(&record, COL_DATE).map_err(|e| err(e))?;
            let trade_date = parse_rbc_date(trade_date_str).map_err(|e| err(e))?;

            let settlement_date_str =
                col_map.get_str(&record, COL_SETTLEMENT_DATE).map_err(|e| err(e))?;
            let settlement_date =
                parse_rbc_date(settlement_date_str).map_err(|e| err(e))?;

            let quantity = parse_decimal(
                col_map.get_str(&record, COL_QUANTITY).map_err(|e| err(e))?,
                COL_QUANTITY,
                row_num,
            )?;
            let price = parse_decimal(
                col_map.get_str(&record, COL_PRICE).map_err(|e| err(e))?,
                COL_PRICE,
                row_num,
            )?;
            let value = parse_decimal(
                col_map.get_str(&record, COL_VALUE).map_err(|e| err(e))?,
                COL_VALUE,
                row_num,
            )?;

            let commission = (value.abs() - (quantity * price).abs()).abs();

            let currency_str =
                col_map.get_str(&record, COL_CURRENCY).map_err(|e| err(e))?;
            let currency = Currency::new(currency_str);

            let account_num_str =
                col_map.get_str(&record, COL_ACCOUNT).map_err(|e| err(e))?;
            // Use the per-row account number, but account type from preamble
            let account_num = if !account_num_str.is_empty() {
                account_num_str.to_string()
            } else {
                preamble_account_num.clone()
            };
            let account = Account {
                broker_name: RBC_DI_BROKER_NAME,
                account_type: preamble_account_type.clone(),
                account_num,
            };

            let memo = account.memo_str();

            let b_tx = BrokerTx {
                security: symbol.to_string(),
                trade_date,
                settlement_date,
                trade_date_and_time: trade_date_str.to_string(),
                settlement_date_and_time: settlement_date_str.to_string(),
                action,
                amount_per_share: price,
                num_shares: quantity.abs(),
                commission,
                currency,
                memo,
                exchange_rate: None,
                affiliate: affiliate.clone(),
                row_num: row_num as u32,
                account,
                sort_tiebreak: None,
                filename: fpath.map(|p| p.to_string_lossy().to_string()),
            };
            txs.push(b_tx);
            Ok(RowResult::Ok)
        })()
        .unwrap_or_else(|e| RowResult::Err(e));

        match row_res {
            RowResult::Ok => {}
            RowResult::Warning(w) => warnings.push(w),
            RowResult::Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        Ok(txs)
    } else {
        Err(SheetToTxsErr {
            txs: Some(txs),
            errors,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_rbc_date() {
        let d = parse_rbc_date("December 24, 2025").unwrap();
        assert_eq!(d, time::macros::date!(2025 - 12 - 24));

        let d = parse_rbc_date("January 2, 2025").unwrap();
        assert_eq!(d, time::macros::date!(2025 - 01 - 02));

        let d = parse_rbc_date("February 18, 2025").unwrap();
        assert_eq!(d, time::macros::date!(2025 - 02 - 18));

        assert!(parse_rbc_date("2025-01-02").is_err());
        assert!(parse_rbc_date("").is_err());
        assert!(parse_rbc_date("Decmber 24, 2025").is_err());
    }

    #[test]
    fn test_parse_preamble_account_info() {
        let preamble = vec![
            "\"Activity Export as of Mar 29, 2026 at 10:51:18 pm ET\"".to_string(),
            "".to_string(),
            "\"Account: 12345678 - RESP Family\"".to_string(),
        ];
        let (num, typ) = parse_preamble_account_info(&preamble).unwrap();
        assert_eq!(num, "12345678");
        assert_eq!(typ, "RESP Family");
    }

    #[test]
    fn test_parse_preamble_no_account() {
        let preamble = vec!["some random text".to_string()];
        assert!(parse_preamble_account_info(&preamble).is_none());
    }

    #[test]
    fn test_affiliate_registered() {
        assert_eq!(
            affiliate_for_account_type("RESP Family"),
            Affiliate::default_registered()
        );
        assert_eq!(
            affiliate_for_account_type("TFSA"),
            Affiliate::default_registered()
        );
        assert_eq!(
            affiliate_for_account_type("RRSP"),
            Affiliate::default_registered()
        );
    }

    #[test]
    fn test_affiliate_non_registered() {
        assert_eq!(affiliate_for_account_type("Cash"), Affiliate::default());
        assert_eq!(affiliate_for_account_type("Margin"), Affiliate::default());
    }

    fn make_csv(rows: &[&str]) -> Vec<u8> {
        let preamble = "\
\"Activity Export as of Mar 29, 2026 at 10:51:18 pm ET\"

\"Account: 12345678 - RESP Family\"

\"Trades this month: 0\"

\"5 Activities\"

\"Date\",\"Activity\",\"Symbol\",\"Symbol Description\",\"Quantity\",\"Price\",\"Settlement Date\",\"Account\",\"Value\",\"Currency\",\"Description\"";
        let mut csv = preamble.to_string();
        for row in rows {
            csv.push('\n');
            csv.push_str(row);
        }
        csv.into_bytes()
    }

    #[test]
    fn test_buy_basic() {
        let csv = make_csv(&[
            "\"December 24, 2025\",\"Buy\",\"XEQT\",\"ISHARES CORE EQUITY ETF\",\"6\",\"40.37\",\"December 29, 2025\",\"12345678\",\"-242.22\",\"CAD\",\"some desc\"",
        ]);
        let txs = csv_to_txs(&csv, None).unwrap();
        assert_eq!(txs.len(), 1);
        let tx = &txs[0];
        assert_eq!(tx.security, "XEQT");
        assert_eq!(tx.action, TxAction::Buy);
        assert_eq!(tx.num_shares, Decimal::from(6));
        assert_eq!(tx.amount_per_share, Decimal::new(4037, 2));
        assert_eq!(tx.trade_date, time::macros::date!(2025 - 12 - 24));
        assert_eq!(tx.settlement_date, time::macros::date!(2025 - 12 - 29));
        assert_eq!(tx.currency, Currency::new("CAD"));
        assert_eq!(tx.affiliate, Affiliate::default_registered());
        // 6 * 40.37 = 242.22, |Value| = 242.22, commission = 0
        assert_eq!(tx.commission, Decimal::ZERO);
    }

    #[test]
    fn test_buy_with_commission() {
        let csv = make_csv(&[
            "\"February 18, 2025\",\"Buy\",\"ZNQ\",\"BMO NASDAQ 100\",\"30\",\"101.10\",\"February 19, 2025\",\"12345678\",\"-3042.95\",\"CAD\",\"desc\"",
        ]);
        let txs = csv_to_txs(&csv, None).unwrap();
        assert_eq!(txs.len(), 1);
        let tx = &txs[0];
        // 30 * 101.10 = 3033.00, |Value| = 3042.95, commission = 9.95
        assert_eq!(tx.commission, Decimal::new(995, 2));
    }

    #[test]
    fn test_sell_basic() {
        let csv = make_csv(&[
            "\"March 15, 2025\",\"Sell\",\"XEQT\",\"ISHARES\",\"10\",\"42.00\",\"March 17, 2025\",\"12345678\",\"410.05\",\"CAD\",\"desc\"",
        ]);
        let txs = csv_to_txs(&csv, None).unwrap();
        assert_eq!(txs.len(), 1);
        let tx = &txs[0];
        assert_eq!(tx.action, TxAction::Sell);
        // 10 * 42.00 = 420.00, Value = 410.05, commission = 9.95
        assert_eq!(tx.commission, Decimal::new(995, 2));
    }

    #[test]
    fn test_silent_skip_activities() {
        let csv = make_csv(&[
            "\"October 2, 2025\",\"Distribution\",\"ZEQT\",\"BMO\",\"\",\"0.07\",\"October 2, 2025\",\"12345678\",\"1.58\",\"CAD\",\"desc\"",
            "\"December 8, 2025\",\"Deposits & Contributions\",\"\",\"\",\"\",\"\",\"December 8, 2025\",\"12345678\",\"1200\",\"CAD\",\"desc\"",
            "\"April 28, 2025\",\"Fees\",\"\",\"\",\"\",\"\",\"April 28, 2025\",\"12345678\",\"-25\",\"CAD\",\"desc\"",
        ]);
        let txs = csv_to_txs(&csv, None).unwrap();
        assert_eq!(txs.len(), 0);
    }

    #[test]
    fn test_warn_skip_activities() {
        let csv = make_csv(&[
            "\"August 15, 2025\",\"Reorganization\",\"ZEQT\",\"BMO\",\"14\",\"\",\"August 19, 2025\",\"12345678\",\"0\",\"CAD\",\"STK SPLIT\"",
            "\"May 2, 2025\",\"Return of Capital\",\"ZNQ\",\"BMO\",\"\",\"\",\"May 2, 2025\",\"12345678\",\"0\",\"CAD\",\"RTC desc\"",
        ]);
        let err = csv_to_txs(&csv, None).unwrap_err();
        let txs = err.txs.unwrap();
        assert_eq!(txs.len(), 0);
        assert_eq!(err.errors.len(), 0);
        assert_eq!(err.warnings.len(), 2);
        assert!(err.warnings[0].to_string().contains("Reorganization"));
        assert!(err.warnings[0].to_string().contains("not automatically converted"));
        assert!(err.warnings[1].to_string().contains("Return of Capital"));
    }

    #[test]
    fn test_unrecognized_activity() {
        let csv = make_csv(&[
            "\"January 5, 2025\",\"Transfer\",\"FOO\",\"desc\",\"10\",\"5.00\",\"January 7, 2025\",\"12345678\",\"-50\",\"CAD\",\"desc\"",
        ]);
        let err = csv_to_txs(&csv, None).unwrap_err();
        assert!(err.errors[0].to_string().contains("Unrecognized activity type"));
    }

    #[test]
    fn test_footer_ignored() {
        let csv = make_csv(&[
            "\"January 3, 2025\",\"Buy\",\"FOO\",\"desc\",\"10\",\"5.00\",\"January 5, 2025\",\"12345678\",\"-50\",\"CAD\",\"desc\"",
            "",
            "\"Disclaimers\"",
            "\"1 This exchange rate is based on...\"",
        ]);
        let txs = csv_to_txs(&csv, None).unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_missing_header() {
        let csv = b"some random data\nno header here\n";
        let err = csv_to_txs(csv, None).unwrap_err();
        assert!(err.txs.is_none());
        assert!(err.errors[0]
            .to_string()
            .contains("Could not find RBC DI CSV header"));
    }

    fn make_csv_with_account(account_type: &str) -> Vec<u8> {
        format!(
            "\
\"Activity Export as of Mar 29, 2026 at 10:51:18 pm ET\"

\"Account: 12345678 - {account_type}\"

\"Trades this month: 0\"

\"5 Activities\"

\"Date\",\"Activity\",\"Symbol\",\"Symbol Description\",\"Quantity\",\"Price\",\"Settlement Date\",\"Account\",\"Value\",\"Currency\",\"Description\"
\"January 3, 2025\",\"Buy\",\"FOO\",\"desc\",\"10\",\"5.00\",\"January 5, 2025\",\"12345678\",\"-50\",\"CAD\",\"desc\""
        )
        .into_bytes()
    }

    #[test]
    fn test_affiliate_from_preamble_registered() {
        for acct_type in &["RRSP", "TFSA", "RESP <kind>", "FHSA", "RRIF"] {
            let csv = make_csv_with_account(acct_type);
            let txs = csv_to_txs(&csv, None).unwrap();
            assert_eq!(
                txs[0].affiliate,
                Affiliate::default_registered(),
                "Expected registered affiliate for account type \"{acct_type}\""
            );
        }
    }

    #[test]
    fn test_affiliate_from_preamble_non_registered() {
        for acct_type in &["Cash", "Margin", ""] {
            let csv = make_csv_with_account(acct_type);
            let txs = csv_to_txs(&csv, None).unwrap();
            assert_eq!(
                txs[0].affiliate,
                Affiliate::default(),
                "Expected default affiliate for account type \"{acct_type}\""
            );
        }
    }

    #[test]
    fn test_account_memo() {
        let csv = make_csv(&[
            "\"January 3, 2025\",\"Buy\",\"FOO\",\"desc\",\"10\",\"5.00\",\"January 5, 2025\",\"12345678\",\"-50\",\"CAD\",\"desc\"",
        ]);
        let txs = csv_to_txs(&csv, None).unwrap();
        assert_eq!(txs[0].memo, "RBC Direct Investing RESP Family 12345678");
        assert_eq!(txs[0].account.broker_name, RBC_DI_BROKER_NAME);
        assert_eq!(txs[0].account.account_num, "12345678");
        assert_eq!(txs[0].account.account_type, "RESP Family");
    }
}
