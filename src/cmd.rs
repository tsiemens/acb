use std::{path::PathBuf, process::ExitCode};
use std::io::Write;

use clap::Parser;

use crate::app::run_acb_app_to_console;
use crate::fx::io::CsvRatesCache;
use crate::util::date::parse_standard_date;
use crate::{app::input_parse::parse_initial_status, portfolio::csv_common::CsvCol, util::rw::{DescribedReader, WriteHandle}, write_errln};

const ABOUT: &str = "Adjusted cost basis (ACB) calculation tool";

fn get_long_about() -> String {
    format!("\
A cli tool which can be used to perform Adjusted cost basis (ACB)
calculations on RSU and stock transactions.

Stocks and transactions can be in other currencies, and conversion rates for
certain currencies* can be automatically downloaded or provided manually.

* Supported conversion rate pairs are:
- CAD/USD

Each CSV provided should contain a header with these column names:
{}
Non-essential columns like exchange rates and currency columns are optional.

Exchange rates are always provided to be multiplied with the given amount to produce
the equivalent value in the default (local) currency.",
        CsvCol::export_order_non_deprecated_cols().join(", "))
}

#[derive(Parser, Debug)]
#[command(version = crate::app::ACB_APP_VERSION,
          about = ABOUT, long_about = get_long_about())]
pub struct Args {
    #[arg(required = true)]
    csv_files: Vec<String>,

    /// Print verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Download exchange rates, even if they are cached
    #[arg(short, long, default_value_t = false)]
    pub force_download: bool,

    /// Format of how dates appear in the csv file.
    /// The default is "[year]-[month]-[day]".
    ///
    /// See https://time-rs.github.io/book/api/well-known-format-descriptions.html
    ///
    /// TODO implement this
    #[arg(long)]
    pub date_fmt: Option<String>,

    /// Base share count and ACBs for symbols, assumed at the beginning of time.
    ///
    /// Formatted as SYM:nShares:totalAcb. Eg. GOOG:20:1000.00 . May be provided multiple times.
    ///
    /// Only applies to the default affiliate.
    #[arg(short = 'b', long)]
    pub symbol_base: Vec<String>,

    /// Print all digits in output values
    #[arg(long, default_value_t = false)]
    pub print_full_values: bool,

    /// Generate a summary CSV for transactions before the provided date
    /// (YYYY-MM-DD format). (--help for more)
    ///
    /// You should include all transactions made up to the
    /// present for an accurate summary.
    #[arg(long)]
    pub summarize_before: Option<String>,

    /// Summary will include transactions which represent annual capital gains/losses.
    ///
    /// Only valid with --summarize-before.
    #[arg(long, default_value_t = false)]
    pub summarize_annual_gains: bool,

    /// Print total costs across all securities (default, non-registered affiliate only)
    #[arg(long, default_value_t = false)]
    pub total_costs: bool,

    /// Write output as CSV to the specified directory.
    #[arg(short = 'd', long)]
    pub csv_output_dir: Option<String>,
}

pub fn command_main() -> Result<(), ExitCode> {
    let args = Args::parse();

    let mut err_printer = WriteHandle::stderr_write_handle();

    let all_init_status = match parse_initial_status(&args.symbol_base) {
        Ok(v) => v,
        Err(e) => {
            write_errln!(err_printer, "Error parsing --symbol-base: {e}");
            return Err(ExitCode::FAILURE);
        },
    };

    let mut csv_readers = Vec::<DescribedReader>::with_capacity(args.csv_files.len());
    for csv_name in args.csv_files {
        let reader = DescribedReader::from_file_path(PathBuf::from(csv_name));
        csv_readers.push(reader);
    }

    let mut options = crate::app::Options::default();

    if let Some(sum_before_date_str) = args.summarize_before {
        options.summary_mode_latest_date = match parse_standard_date(&sum_before_date_str) {
            Ok(d) => Some(d),
            Err(e) => {
                write_errln!(err_printer, "Error: {e}");
                return Err(ExitCode::FAILURE);
            },
        };
    }

    let home_dir = match crate::util::sys::home_dir_path() {
        Ok(d) => d,
        Err(e) => {
            write_errln!(err_printer, "Unable to determine user home directory: {e}");
            return Err(ExitCode::FAILURE);
        },
    };

    let rates_cache = Box::new(CsvRatesCache::new(home_dir, err_printer.clone()));

    run_acb_app_to_console(
        csv_readers, all_init_status, options, rates_cache,
        err_printer)
    .map_err(|_| ExitCode::FAILURE)
}