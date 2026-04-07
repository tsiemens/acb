use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use regex::Regex;
use rust_decimal::Decimal;

use crate::app::config::AcbConfig;
use crate::app::outfmt::csv::CsvWriter;
use crate::app::outfmt::model::AcbWriter;
use crate::app::outfmt::text::TextWriter;
use crate::peripheral::broker::Account;
use crate::peripheral::broker::{FileDetectSource, FileKind};
use crate::portfolio::CsvTx;
use crate::portfolio::Currency;
use crate::util::basic::SError;
use crate::util::date::DateRange;
use crate::util::rw::WriteHandle;
use crate::write_errln;

use super::broker::questrade;
use super::broker::rbc_di;
use super::broker::BrokerTx;
use super::excel::{read_xl_source, XlSource};

fn filter_and_verify_tx_accounts(
    account_filter: &Option<Regex>,
    txs: Vec<BrokerTx>,
) -> Result<Vec<BrokerTx>, SError> {
    if let Some(filt) = account_filter {
        Ok(txs
            .into_iter()
            .filter(|tx| filt.is_match(&tx.account.account_str()))
            .collect())
    } else {
        let accounts: HashSet<&Account> =
            HashSet::from_iter(txs.iter().map(|tx| &tx.account));
        if accounts.len() > 1 {
            let accounts_str = accounts
                .iter()
                .map(|ac| ac.account_str())
                .collect::<Vec<String>>()
                .join(", ");
            Err(format!(
                "No account was specified, and found transactions for \
                multiple accounts ({accounts_str}). \
                If you wish to include all accounts, provide --account=."
            ))
        } else {
            Ok(txs)
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum BrokerArg {
    Questrade,
    RbcDi,
}

impl std::fmt::Display for BrokerArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{self:?}").to_lowercase();
        write!(f, "{s}")
    }
}

fn detect_broker(path: &std::path::Path) -> Result<BrokerArg, SError> {
    let result = super::broker::detect_file_kind(FileDetectSource::Path(path))?;
    match result.kind {
        FileKind::QuestradeExcel => Ok(BrokerArg::Questrade),
        FileKind::RbcDiCsv => Ok(BrokerArg::RbcDi),
        _ => {
            let hint = result.warning.map(|w| format!(" ({w})")).unwrap_or_default();
            Err(format!(
                "Could not auto-detect broker for \"{}\"{hint}. \
                 Please specify --broker.",
                path.display()
            ))
        }
    }
}

struct BrokerResult {
    txs: Vec<BrokerTx>,
    non_fatal_errors: Vec<String>,
    warnings: Vec<String>,
}

/// Unwraps a broker parser result into a `BrokerResult`, or returns
/// a fatal `Err` when no partial output is available.
fn unwrap_broker_result(
    tx_res: Result<Vec<BrokerTx>, super::broker::SheetToTxsErr>,
) -> Result<BrokerResult, SError> {
    match tx_res {
        Ok(txs) => Ok(BrokerResult {
            txs,
            non_fatal_errors: vec![],
            warnings: vec![],
        }),
        Err(res_err) => {
            if let Some(partial_txs) = res_err.txs {
                let errs = res_err.errors.iter().map(|e| format!("{e}")).collect();
                let warns =
                    res_err.warnings.iter().map(|e| format!("{e}")).collect();
                Ok(BrokerResult {
                    txs: partial_txs,
                    non_fatal_errors: errs,
                    warnings: warns,
                })
            } else {
                // Fatal error. No partial output.
                let errs: Vec<String> =
                    res_err.errors.iter().map(|e| format!("{e}")).collect();
                Err(errs.join("\n"))
            }
        }
    }
}

/// Applies common post-processing to broker transactions: account/security
/// filtering, config-based affiliate override, FX filtering, exchange rate
/// override, sorting, and conversion to CsvTx.
fn process_broker_txs(
    mut txs: Vec<BrokerTx>,
    account_filter: Option<Regex>,
    security_filter: Option<Regex>,
    no_fx: bool,
    no_sort: bool,
    usd_exchange_rate: Option<Decimal>,
    config: Option<&AcbConfig>,
) -> Result<Vec<CsvTx>, SError> {
    txs = filter_and_verify_tx_accounts(&account_filter, txs)?;

    if let Some(pattern) = security_filter {
        txs = txs.into_iter().filter(|tx| pattern.is_match(&tx.security)).collect();
    }
    if no_fx {
        txs = txs.into_iter().filter(|tx| !tx.security.ends_with(".FX")).collect();
    }
    if let Some(rate) = usd_exchange_rate {
        for tx in &mut txs {
            if tx.currency == Currency::usd() {
                tx.exchange_rate = Some(rate)
            }
        }
    }
    if !no_sort {
        txs.sort();
    }

    Ok(txs
        .into_iter()
        .map(|t| {
            let af =
                super::broker::affiliate_for_account_with_config(&t.account, config);
            t.to_csv_tx(af)
        })
        .collect())
}

pub struct ConvertResult {
    pub csv_txs: Vec<CsvTx>,
    pub non_fatal_errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Converts an Excel workbook into a list of CSV transactions.
///
/// Returns a `ConvertResult` with `csv_txs` (possibly partial) plus any
/// non-fatal `errors` and `warnings`.
/// Returns `Err` only on fatal errors where no output can be produced.
pub fn convert_xl_txs(
    source: XlSource,
    broker: &BrokerArg,
    sheet: Option<&str>,
    account_filter: Option<Regex>,
    security_filter: Option<Regex>,
    no_fx: bool,
    no_sort: bool,
    usd_exchange_rate: Option<Decimal>,
    config: Option<&AcbConfig>,
) -> Result<ConvertResult, SError> {
    let source_path: Option<PathBuf> = match &source {
        XlSource::Path(p) => Some(p.clone()),
        XlSource::Data(_) => None,
    };

    let rg = read_xl_source(source, sheet)?;

    let tx_res = match broker {
        BrokerArg::Questrade => questrade::sheet_to_txs(&rg, source_path.as_deref()),
        BrokerArg::RbcDi => {
            return Err("RBC DI uses CSV format, not Excel. \
                        Please provide a .csv file."
                .to_string())
        }
    };

    let broker_res = unwrap_broker_result(tx_res)?;
    let csv_txs = process_broker_txs(
        broker_res.txs,
        account_filter,
        security_filter,
        no_fx,
        no_sort,
        usd_exchange_rate,
        config,
    )?;
    Ok(ConvertResult {
        csv_txs,
        non_fatal_errors: broker_res.non_fatal_errors,
        warnings: broker_res.warnings,
    })
}

/// Converts a CSV broker export file into a list of CSV transactions.
///
/// Returns a `ConvertResult` with `csv_txs` (possibly partial) plus any
/// non-fatal `errors` and `warnings`.
/// Returns `Err` only on fatal errors where no output can be produced.
pub fn convert_csv_broker_txs(
    csv_data: &[u8],
    fpath: Option<&std::path::Path>,
    account_filter: Option<Regex>,
    security_filter: Option<Regex>,
    no_fx: bool,
    no_sort: bool,
    usd_exchange_rate: Option<Decimal>,
    config: Option<&AcbConfig>,
) -> Result<ConvertResult, SError> {
    let tx_res = rbc_di::csv_to_txs(csv_data, fpath);

    let broker_res = unwrap_broker_result(tx_res)?;
    let csv_txs = process_broker_txs(
        broker_res.txs,
        account_filter,
        security_filter,
        no_fx,
        no_sort,
        usd_exchange_rate,
        config,
    )?;
    Ok(ConvertResult {
        csv_txs,
        non_fatal_errors: broker_res.non_fatal_errors,
        warnings: broker_res.warnings,
    })
}

/// A convenience script to convert export files from brokerages to the ACB
/// transaction csv format.
/// Supports Questrade (.xlsx) and RBC Direct Investing (.csv).
#[derive(Parser, Debug)]
#[command(author, about)]
pub struct Args {
    /// Table file exported from your brokerage platform.
    /// A .xlsx for Questrade, a .csv for RBC Direct Investing.
    #[arg(required = true)]
    pub export_file: PathBuf,

    #[arg(long, default_value_t = false)]
    pub no_sort: bool,

    /// Broker whose export format to parse. If omitted, auto-detected from
    /// the file contents.
    #[arg(short = 'b', long, ignore_case = true)]
    pub broker: Option<BrokerArg>,

    /// Specify an exchange rate to use.
    ///
    /// May be useful if rates for more recent transactions may not
    /// be posted yet
    #[arg(long)]
    pub usd_exchange_rate: Option<Decimal>,

    /// Specify the account to export transactions for.
    ///
    /// Must partially match, and is treated as a regular-expression.
    ///
    /// The account is formatted into '{account type} {account number}'.
    #[arg(short = 'a', long)]
    pub account: Option<Regex>,

    /// Filter output rows to contain only symbol/security.
    ///
    /// Is treated as a regular expression
    #[arg(long, alias = "symbol")]
    pub security: Option<Regex>,

    // Do not generate transactions for foreign currency exchanges
    #[arg(long, default_value_t = false)]
    pub no_fx: bool,

    #[arg(long)]
    pub pretty: bool,

    /// Select which sheet (by name) in the spreadsheet file to use.
    #[arg(long)]
    pub sheet: Option<String>,

    /// Only include transactions whose settlement date falls in this year.
    #[arg(long)]
    pub year: Option<i32>,

    /// Path to acb-config.json. Defaults to the platform config dir.
    ///
    ///   Linux:   ~/.config/acb/acb-config.json
    ///
    ///   macOS:   ~/Library/Application Support/acb/acb-config.json
    ///
    ///   Windows: %APPDATA%\acb\acb-config.json
    #[arg(long)]
    pub config: Option<PathBuf>,
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();
    run_with_args(
        args,
        WriteHandle::stdout_write_handle(),
        WriteHandle::stderr_write_handle(),
    )
}

pub fn run_with_args(
    args: Args,
    out_w: WriteHandle,
    mut err_w: WriteHandle,
) -> Result<(), ()> {
    let date_range = args.year.map(DateRange::for_year);

    // Load config
    let config_path =
        args.config.or_else(|| crate::app::config::default_config_path());
    let config = match config_path {
        Some(ref p) => crate::app::config::load_config(p)
            .map_err(|e| write_errln!(err_w, "{e}"))?,
        None => None,
    };

    let broker = match args.broker {
        Some(b) => b,
        None => match detect_broker(&args.export_file) {
            Ok(b) => b,
            Err(e) => {
                write_errln!(err_w, "{e}");
                return Err(());
            }
        },
    };

    let convert_result = match broker {
        BrokerArg::RbcDi => {
            let data = std::fs::read(&args.export_file).map_err(|e| {
                write_errln!(
                    err_w,
                    "Failed to read {}: {e}",
                    args.export_file.display()
                );
            })?;
            convert_csv_broker_txs(
                &data,
                Some(args.export_file.as_path()),
                args.account,
                args.security,
                args.no_fx,
                args.no_sort,
                args.usd_exchange_rate,
                config.as_ref(),
            )
        }
        BrokerArg::Questrade => convert_xl_txs(
            XlSource::Path(args.export_file.clone()),
            &broker,
            args.sheet.as_deref(),
            args.account,
            args.security,
            args.no_fx,
            args.no_sort,
            args.usd_exchange_rate,
            config.as_ref(),
        ),
    };

    let convert_res = match convert_result {
        Ok(result) => result,
        Err(e) => {
            write_errln!(err_w, "{e}");
            return Err(());
        }
    };

    let csv_txs = if let Some(ref range) = date_range {
        convert_res
            .csv_txs
            .into_iter()
            .filter(|tx| tx.settlement_date.map_or(false, |d| range.contains(&d)))
            .collect()
    } else {
        convert_res.csv_txs
    };

    let mut printer: Box<dyn AcbWriter> = if args.pretty {
        Box::new(TextWriter::new(out_w))
    } else {
        Box::new(CsvWriter::new_to_writer(out_w))
    };
    let table_title = format!("{} TXs", args.export_file.display());
    let csv_file_name = format!("{}_txs.csv", args.export_file.display());
    let csv_table = crate::portfolio::io::tx_csv::txs_to_csv_table(&csv_txs);
    printer
        .print_render_table(
            &table_title,
            &csv_file_name,
            &crate::portfolio::render::RenderTable::from(csv_table),
        )
        .map_err(|e| {
            write_errln!(err_w, "{e}");
        })?;

    if !convert_res.warnings.is_empty() {
        let _ = write!(err_w, "Warnings:");
        for w in &convert_res.warnings {
            write_errln!(err_w, " - {w}");
        }
    }
    if !convert_res.non_fatal_errors.is_empty() {
        let _ = write!(err_w, "Errors:");
        for e in &convert_res.non_fatal_errors {
            write_errln!(err_w, " - {e}");
        }
        Err(())
    } else {
        Ok(())
    }
}
