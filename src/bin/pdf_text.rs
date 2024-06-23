use std::{io::Write, path::PathBuf};

use clap::Parser;

use acb::peripheral::pdf;

/// This is a simple wrapper around the pdf parser library to help generate test
/// files for any pdf-handling logic.
/// Contributors should manually sanitize sensitive information before committing to
/// the repo as a test file.
/// It is recommended to keep copies of the original PDFs somewhere, so the text
/// can be regenerated if it is noticed that the behaviour of pdf_extract changes in
/// a material way.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
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
}

/// page_num should be one-based
fn page_marker_line(page_num: u32) -> String {
    format!("---------- Page {page_num} ----------")
}

fn main() -> Result<(), ()> {
    let args = Args::parse();

    let mut pdf_doc = lopdf::Document::load(args.input).unwrap();

    if args.show_page_numbers || args.parsable_page_markers {
        let mut has_printed = false;
        for (i, _) in pdf_doc.page_iter().enumerate() {
            let page_num = ((i + 1)).try_into().unwrap();
            if let Some(pages_to_show) = &args.pages {
                if !pages_to_show.contains(&page_num) {
                    continue;
                }
            }

            if args.parsable_page_markers {
                print!("{}", pdf::parseable_page_marker(page_num));
            } else {
                println!("{}{}",
                    if !has_printed { "" } else { "\n" },
                    page_marker_line(page_num));
            }
            pdf::write_page_text(&pdf_doc, page_num, &mut std::io::stdout()).unwrap();
            std::io::stdout().flush().unwrap();
            has_printed = true;
        }
    } else {
        if let Some(pages_to_show) = args.pages {
            pdf::filter_pdf_pages(&mut pdf_doc, &pages_to_show);
        }
        pdf::write_doc_text(&pdf_doc, &mut std::io::stdout()).unwrap();
    }

    Ok(())
}
