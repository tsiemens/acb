use acb::util::basic::SError;
use clap::Parser;

/// A convenience script to merge CSV files to stdout
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// One or more CSV files. Rows are written in the provided order.
    #[arg(required = true)]
    pub csv_files: Vec<String>,
}

fn main() -> Result<(), SError> {
    let args = Args::parse();

    acb::peripheral::csv_merge_impl::merge_csv_files(
        &args.csv_files,
        std::io::stdout(),
    )
}
