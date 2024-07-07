use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use regex::Regex;
use rust_decimal::Decimal;

use office::{Excel, Range};

use crate::peripheral::broker::Account;
use crate::portfolio::Currency;
use crate::util::basic::SError;
use crate::util::rw::WriteHandle;
use crate::write_errln;

use super::broker::questrade;
use super::broker::BrokerTx;

/// Reads the named sheet or the only sheet.
/// If no sheet name is provided, there must only be a single sheet,
/// otherwise returns Err.
///
/// Note: This could not/cannot be based on the sheet index,
/// because the office library does not provide an API to get the
/// sheets in any particular order. They end up coming back in a random
/// order.
fn read_xl_file(path: &Path, sheet_name: Option<&str>) -> Result<Range, SError> {
    let mut workbook = Excel::open(path).map_err(|e| format!("{e}"))?;

    let sheet_names: Vec<String>;

    let sheet = if let Some(sn) = sheet_name {
        sn
    } else {
        sheet_names = workbook.sheet_names().map_err(|e| format!("{e}"))?;
        if sheet_names.len() > 1 {
            return Err(format!(
                "Workbook has more than one one sheet: {sheet_names:?}. \
                Sheet name must be specified"))
        }
        sheet_names.get(0).ok_or_else(|| "Workbook has no sheets".to_string())?
    };

    workbook.worksheet_range(sheet).map_err(|e| format!("{e}"))
}

fn filter_and_verify_tx_accounts(account_filter: &Option<Regex>, txs: Vec<BrokerTx>)
 -> Result<Vec<BrokerTx>, SError> {
    if let Some(filt) = account_filter {
        Ok(txs.into_iter().filter(
            |tx| filt.is_match(&tx.account.account_str()))
            .collect())
    } else {
        let accounts: HashSet<&Account> = HashSet::from_iter(
            txs.iter().map(|tx| &tx.account)
        );
        if accounts.len() > 1 {
            let accounts_str = accounts.iter().map(|ac| ac.account_str())
                .collect::<Vec<String>>().join(", ");
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
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();
    run_with_args(
        args,
        WriteHandle::stdout_write_handle(),
        WriteHandle::stderr_write_handle())
}

pub fn run_with_args(args: Args, mut out_w: WriteHandle, mut err_w: WriteHandle)
-> Result<(), ()>  {
    let rg = match read_xl_file(&args.export_file,
                                args.sheet.as_ref().map(|v| v.as_str())) {
        Ok(rg) => rg,
        Err(e) => {
            write_errln!(err_w, "{e}");
            return Err(());
        },
    };

    let tx_res = match args.broker {
        BrokerArg::Questrade => questrade::sheet_to_txs(
            &rg, Some(&args.export_file)),
    };

    let (mut txs, errors) = match tx_res {
        Ok(txs) => (txs, None),
        Err(res_err) => {
            if let Some(partial_txs) = res_err.txs {
                (partial_txs, Some(res_err.errors))
            } else {
                // Fatal error. No partial output.
                let _ = write!(err_w, "Error:");
                for e in res_err.errors {
                    write_errln!(err_w, "{e}");
                }
                return Err(());
            }
        },
    };

    txs = match filter_and_verify_tx_accounts(&args.account, txs) {
        Ok(txs_) => txs_,
        Err(e) => {
            write_errln!(err_w, "{e}");
            return Err(());
        },
    };

    if let Some(pattern) = args.security {
        txs = txs.into_iter().filter(|tx| pattern.is_match(&tx.security)).collect();
    }
    if args.no_fx {
        txs = txs.into_iter().filter(|tx| !tx.security.ends_with(".FX")).collect();
    }
    if let Some(rate) = args.usd_exchange_rate {
        for tx in &mut txs {
            if tx.currency == Currency::usd() {
                tx.exchange_rate = Some(rate)
            }
        }
    }
    if !args.no_sort {
        txs.sort();
    }

    // Convert to CsvTx (at least. Maybe even to Tx and back too)
    let csv_txs: Vec<crate::portfolio::CsvTx> = txs.into_iter().map(|t| t.into())
        .collect();

    if args.pretty {
        todo!();
    } else {
        match crate::portfolio::io::tx_csv::write_txs_to_csv(&csv_txs, &mut out_w) {
            Ok(()) => (),
            Err(e) => {
                write_errln!(err_w, "{e}");
                return Err(());
            },
        }
    }

    if let Some(es) = errors {
        let _ = write!(err_w, "Errors:");
        for e in es {
            write_errln!(err_w, " - {e}");
        }
        Err(())
    } else {
        Ok(())
    }
}