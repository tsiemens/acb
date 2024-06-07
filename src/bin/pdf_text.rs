use std::path::PathBuf;

use clap::Parser;

/// This is a simple wrapper around the pdf parser library to help generate test
/// files for any pdf-handling logic.
/// Contributors should manually sanitize sensitive information before committing to
/// the repo as a test file.
/// It is recommended to keep copies of the original PDFs somewhere, so the text
/// can be regenerated if it is noticed that the behaviour of pdf_extract changes in
/// a material way.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file
    #[arg(required = true)]
    input: PathBuf,
}

fn main() -> Result<(), ()> {
    let args = Args::parse();

    let bytes = std::fs::read(args.input).unwrap();
    let out = pdf_extract::extract_text_from_mem(&bytes).unwrap();
    println!("{out}");

    Ok(())
}
