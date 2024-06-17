use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use regex::Regex;
use rust_decimal::Decimal;

use office::{Excel, Range};

use crate::peripheral::broker::Account;
use crate::portfolio::Currency;
use crate::util::basic::SError;

use super::broker::questrade;
use super::broker::BrokerTx;

/// Sheet here is a 1-based index.
fn read_xl_file(path: &Path, sheet: usize) -> Result<Range, SError> {
    let mut workbook = Excel::open(path).map_err(|e| format!("{e}"))?;
    let sheet_index = sheet - 1;

    let sheet_names = workbook.sheet_names().map_err(|e| format!("{e}"))?;
    let sheet_name = sheet_names.get(sheet_index).ok_or(format!("No sheet {sheet_index}"))?;

    workbook.worksheet_range(sheet_name).map_err(|e| format!("{e}"))
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
enum BrokerArg {
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
#[command(author, version, about)]
struct Args {
    /// Table file exported from your brokerage platform.
    /// A .xlsx for Questrade
    #[arg(required = true)]
    export_file: PathBuf,

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

    /// Select which sheet in the spreadsheet file to use.
    #[arg(long, default_value_t = 1)]
    pub sheet: usize,
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();

    let rg = match read_xl_file(&args.export_file, args.sheet) {
        Ok(rg) => rg,
        Err(e) => {
            eprintln!("{e}");
            return Err(());
        },
    };

    let tx_res = match args.broker {
        BrokerArg::Questrade => questrade::sheet_to_txs(&rg),
    };

    let (mut txs, errors) = match tx_res {
        Ok(txs) => (txs, None),
        Err(res_err) => {
            if let Some(partial_txs) = res_err.txs {
                (partial_txs, Some(res_err.errors))
            } else {
                // Fatal error. No partial output.
                eprint!("Error:");
                for e in res_err.errors {
                    eprintln!("{e}");
                }
                return Err(());
            }
        },
    };

    txs = match filter_and_verify_tx_accounts(&args.account, txs) {
        Ok(txs_) => txs_,
        Err(e) => {
            eprintln!("{e}");
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
        match crate::portfolio::io::tx_csv::write_txs_to_csv(
            &csv_txs, &mut std::io::stdout()) {
                Ok(()) => (),
                Err(e) => {
                    eprintln!("{e}");
                    return Err(());
                },
            }
    }

    if let Some(es) = errors {
        eprintln!("Errors:");
        for e in es {
            eprintln!(" - {e}");
        }
        Err(())
    } else {
        Ok(())
    }
}