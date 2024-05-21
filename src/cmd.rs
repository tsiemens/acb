use clap::Parser;

use crate::portfolio::csv_common::CsvCol;

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

pub fn command_main() {
    let args = Args::parse();

    println!("{:#?}", args);

    crate::app::print_dummy_table();
}