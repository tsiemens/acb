use std::io::Write;
use std::{path::PathBuf, process::ExitCode};

use clap::Parser;

use crate::app::run_acb_app_to_console;
use crate::fx::io::{CsvRatesCache, JsonRemoteRateLoader, RateLoader};
use crate::portfolio::io::tx_csv::TxCsvParseOptions;
use crate::util::date::{parse_dyn_date_format, parse_standard_date};
use crate::util::http::standalone::StandaloneAppRequester;
use crate::{
    portfolio::csv_common::CsvCol,
    util::rw::{DescribedReader, WriteHandle},
    write_errln,
};

const ABOUT: &str = "Adjusted cost basis (ACB) calculation tool";

fn get_long_about() -> String {
    format!(
        "\
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
        CsvCol::export_order_non_deprecated_cols().join(", ")
    )
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
    #[arg(long)]
    pub date_fmt: Option<String>,

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

    /// Write output as a zip file of containing CSVs
    #[arg(short = 'z', long)]
    pub csv_output_zip: Option<String>,
}

pub fn command_main() -> Result<(), ExitCode> {
    crate::tracing::setup_tracing();

    let args = Args::parse();

    if args.verbose {
        crate::log::set_verbose(true);
    }

    let mut err_printer = WriteHandle::stderr_write_handle();

    let mut csv_readers =
        Vec::<DescribedReader>::with_capacity(args.csv_files.len());
    for csv_name in args.csv_files {
        let reader = DescribedReader::from_file_path(PathBuf::from(csv_name));
        csv_readers.push(reader);
    }

    let csv_parse_options = TxCsvParseOptions {
        date_format: match args.date_fmt {
            Some(fmt) => match parse_dyn_date_format(fmt.as_str()) {
                Ok(f) => Some(f),
                Err(e) => {
                    write_errln!(err_printer, "Error parsing --date-fmt: {e}");
                    return Err(ExitCode::FAILURE);
                }
            },
            None => None,
        },
    };

    let mut options = crate::app::Options {
        render_full_dollar_values: args.print_full_values,
        summary_mode_latest_date: None, // set below
        split_annual_summary_gains: args.summarize_annual_gains,
        render_total_costs: args.total_costs,
        csv_output_dir: args.csv_output_dir,
        csv_output_zip: args.csv_output_zip,
        csv_parse_options: csv_parse_options,
    };

    if let Some(sum_before_date_str) = args.summarize_before {
        options.summary_mode_latest_date =
            match parse_standard_date(&sum_before_date_str) {
                Ok(d) => Some(d),
                Err(e) => {
                    write_errln!(err_printer, "Error: {e}");
                    return Err(ExitCode::FAILURE);
                }
            };
    }

    let home_dir = match crate::util::os::home_dir_path() {
        Ok(d) => d,
        Err(e) => {
            write_errln!(
                err_printer,
                "Unable to determine user home directory: {e}"
            );
            return Err(ExitCode::FAILURE);
        }
    };

    let rate_loader = RateLoader::new(
        args.force_download,
        Box::new(CsvRatesCache::new(home_dir, err_printer.clone())),
        JsonRemoteRateLoader::new_boxed(StandaloneAppRequester::new_boxed()),
        err_printer.clone(),
    );

    async_std::task::block_on(run_acb_app_to_console(
        csv_readers,
        options,
        rate_loader,
        err_printer,
    ))
    .map_err(|_| ExitCode::FAILURE)
}
