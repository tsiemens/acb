use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use regex::Regex;
use rust_decimal::Decimal;

use crate::app::outfmt::csv::CsvWriter;
use crate::app::outfmt::model::AcbWriter;
use crate::app::outfmt::text::TextWriter;
use crate::peripheral::broker::Account;
use crate::portfolio::CsvTx;
use crate::portfolio::Currency;
use crate::util::basic::SError;
use crate::util::date::DateRange;
use crate::util::rw::WriteHandle;
use crate::write_errln;

use super::broker::questrade;
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
}

impl std::fmt::Display for BrokerArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{self:?}").to_lowercase();
        write!(f, "{s}")
    }
}

/// Converts an Excel workbook into a list of CSV transactions.
///
/// Returns `(csv_txs, non_fatal_errors)`. When `non_fatal_errors` is non-empty,
/// `csv_txs` is a partial result; callers should report the errors to the user.
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
) -> Result<(Vec<CsvTx>, Vec<String>), SError> {
    let source_path: Option<PathBuf> = match &source {
        XlSource::Path(p) => Some(p.clone()),
        XlSource::Data(_) => None,
    };

    let rg = read_xl_source(source, sheet)?;

    let tx_res = match broker {
        BrokerArg::Questrade => questrade::sheet_to_txs(&rg, source_path.as_deref()),
    };

    let (mut txs, non_fatal_errors) = match tx_res {
        Ok(txs) => (txs, vec![]),
        Err(res_err) => {
            if let Some(partial_txs) = res_err.txs {
                let errs = res_err.errors.iter().map(|e| format!("{e}")).collect();
                (partial_txs, errs)
            } else {
                // Fatal error. No partial output.
                let errs: Vec<String> =
                    res_err.errors.iter().map(|e| format!("{e}")).collect();
                return Err(errs.join("\n"));
            }
        }
    };

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

    let csv_txs: Vec<CsvTx> = txs.into_iter().map(|t| t.into()).collect();
    Ok((csv_txs, non_fatal_errors))
}

/// A convenience script to convert export spreadsheets from brokerages to the ACB
/// transaction csv format.
/// Currently only supports Questrade.
#[derive(Parser, Debug)]
#[command(author, about)]
pub struct Args {
    /// Table file exported from your brokerage platform.
    /// A .xlsx for Questrade
    #[arg(required = true)]
    pub export_file: PathBuf,

    #[arg(long, default_value_t = false)]
    pub no_sort: bool,

    #[arg(short = 'b', long, default_value_t = BrokerArg::Questrade,
          ignore_case = true)]
    pub broker: BrokerArg,

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

    let (csv_txs, non_fatal_errors) = match convert_xl_txs(
        XlSource::Path(args.export_file.clone()),
        &args.broker,
        args.sheet.as_deref(),
        args.account,
        args.security,
        args.no_fx,
        args.no_sort,
        args.usd_exchange_rate,
    ) {
        Ok(result) => result,
        Err(e) => {
            write_errln!(err_w, "{e}");
            return Err(());
        }
    };

    let csv_txs = if let Some(ref range) = date_range {
        csv_txs
            .into_iter()
            .filter(|tx| tx.settlement_date.map_or(false, |d| range.contains(&d)))
            .collect()
    } else {
        csv_txs
    };

    let mut printer: Box<dyn AcbWriter> = if args.pretty {
        Box::new(TextWriter::new(out_w))
    } else {
        Box::new(CsvWriter::new_to_writer(out_w))
    };
    let table_name = if args.pretty {
        format!("{} TXs", args.export_file.display())
    } else {
        format!("{}_txs", args.export_file.display())
    };
    let csv_table = crate::portfolio::io::tx_csv::txs_to_csv_table(&csv_txs);
    printer
        .print_render_table(
            crate::app::outfmt::model::OutputType::Raw,
            &table_name,
            &crate::portfolio::render::RenderTable::from(csv_table),
        )
        .map_err(|e| {
            write_errln!(err_w, "{e}");
        })?;

    if !non_fatal_errors.is_empty() {
        let _ = write!(err_w, "Errors:");
        for e in &non_fatal_errors {
            write_errln!(err_w, " - {e}");
        }
        Err(())
    } else {
        Ok(())
    }
}
