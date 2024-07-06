use std::path::PathBuf;

use clap::Parser;

use super::broker::etrade::BenefitEntry;

/// A convenience script to extract transactions from PDFs downloaded from
/// us.etrade.com
///
/// Instructions:
/// Go to us.etrade.com, log into your account, and go to 'At Work', then to
/// 'Holdings'. In ESPP and RS sections, click 'Benefit History'. Expand each relevant
/// section, and donwload (right-click and 'save link as') each
/// 'View confirmation of purchase' or 'View confirmation of release' link PDF.
///
/// Then go to 'Account', then 'Documents' > 'Trade Confirmations.' Adjust the date
/// range, and download the trade confirmation PDF for each sale.
/// Note: For sales on the same day, both appear on the same PDF. The download link
/// for both sales is to the same document, so only one needs to be downloaded.
///
/// Run this script, giving the name of all PDFs as arguments.
#[derive(Parser, Debug)]
#[command(author, about, long_about)]
struct Args {
    /// ETRADE statement PDFs
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Print pretty tables instead of CSV
    #[arg(short = 'p', long)]
    pub pretty: bool,

    /// Do not try to harmonize trade confirmations with benefit history.
    /// Simply extract and dump them out separately.
    #[arg(long)]
    pub extract_only: bool,

    /// Turn on some very verbose debug printing
    ///
    /// Does not affect tracing. Set TRACE variable for this.
    #[arg(long)]
    pub debug: bool,
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();

    crate::tracing::setup_tracing();

    let mut benefits: Vec<BenefitEntry> = Vec::new();
    for (i, fpath) in args.files.iter().enumerate() {
        if i != 0 {
            if args.debug {
                // Line separator between entries
                eprintln!()
            }
        }
        if args.debug {
            eprintln!("Parsing {fpath:?}");
        }
        use crate::peripheral::pdf;
        use crate::peripheral::broker::etrade;
        let pdf_text = pdf::get_all_pages_text_from_path(fpath)
            .map_err(|e| eprintln!("Failed to read {fpath:?}: {e}"))?
            .join("\n");
        let pdf_content = etrade::parse_pdf_text(&pdf_text, fpath)
            .map_err(|e| eprintln!("Failed to parse {fpath:?}: {e}"))?;
        match pdf_content {
            etrade::EtradePdfContent::BenefitConfirmation(mut bs) => {
                if args.debug {
                    eprintln!("{bs:#?}");
                }
                benefits.append(&mut bs);
            },
            etrade::EtradePdfContent::TradeConfirmation(_) => todo!(),
        }
    }

    Ok(())
}