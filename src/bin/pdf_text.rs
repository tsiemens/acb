use std::path::PathBuf;

use clap::Parser;

use acb::peripheral::pdf;

#[derive(clap::ValueEnum, PartialEq, Clone, Debug)]
pub enum ReaderArg {
    Auto,
    Pypdf,
    Lopdf,
}

impl std::fmt::Display for ReaderArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{self:?}").to_lowercase();
        write!(f, "{s}")
    }
}

/// This is a simple wrapper around the pdf parser library to help generate test
/// files for any pdf-handling logic.
///
/// Contributors should manually sanitize sensitive information before committing to
/// the repo as a test file.
/// It is recommended to keep copies of the original PDFs somewhere, so the text
/// can be regenerated if it is noticed that the behaviour of pdf_extract changes in
/// a material way.
#[derive(Parser, Debug)]
#[command(author, about, long_about)]
struct Args {
    /// Input file
    #[arg(required = true)]
    pub input: PathBuf,

    /// Show pretty page deliniators
    #[arg(short = 'n', long)]
    pub show_page_numbers: bool,

    /// Inserts page deliniators that can be parsed back out.
    /// Generally for generating test data.
    #[arg(short = 'm', long)]
    pub parsable_page_markers: bool,

    /// Can be provided multiple times
    #[arg(short = 'p', long = "page", value_name = "PAGE")]
    pub pages: Option<Vec<u32>>,

    /// Select a PDF reader library to use
    ///
    /// Some PDFs are not compatible with all reader libraries. "auto" will
    /// try to find a reader which is compatible with the provided PDF.
    #[arg(short = 'r', long = "reader", value_name = "PDF_READER",
          default_value_t=ReaderArg::Auto)]
    pub reader: ReaderArg
}

/// page_num should be one-based
fn page_marker_line(page_num: u32) -> String {
    format!("---------- Page {page_num} ----------")
}

fn main() -> Result<(), ()> {
    let args = Args::parse();

    let page_getter = match args.reader {
        ReaderArg::Auto => pdf::get_pages_text_from_path,
        ReaderArg::Pypdf => pdf::get_pages_text_from_path_py,
        ReaderArg::Lopdf => pdf::get_pages_text_from_path_lo,
    };

    let path = args.input;
    let pages_refs: Option<&Vec<u32>> = args.pages.as_ref().map(|v| v);

    let pages = page_getter(&path, pages_refs.clone().map(|v| v.as_ref()))
        .map_err(|e| eprintln!("Error getting pages from {path:?}: {e}"))?;
    for (i, page) in pages.iter().enumerate() {
        let page_num: u32 = match pages_refs {
            Some(pnums) => pnums.get(i).map(|n| *n).unwrap_or(0),
            None => i as u32 + 1,
        };
        if args.parsable_page_markers {
            print!("{}", pdf::parseable_page_marker(page_num));
        } else if args.show_page_numbers {
            println!("{}{}",
                if i == 0 { "" } else { "\n" },
                page_marker_line(page_num));
        }
        if args.parsable_page_markers {
            print!("{}", page);
        } else {
            println!("{}", page);
        }
    }

    Ok(())
}
