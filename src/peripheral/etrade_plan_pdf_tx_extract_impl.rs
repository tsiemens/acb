use std::path::PathBuf;

use clap::Parser;

use crate::app::outfmt::csv::CsvWriter;
use crate::app::outfmt::model::{AcbWriter, OutputType};
use crate::app::outfmt::text::TextWriter;
use crate::portfolio::render::RenderTable;
use crate::util::rw::WriteHandle;
use crate::{peripheral::pdf, util::basic::SError};
use crate::peripheral::broker::etrade;
use super::broker::{etrade::BenefitEntry, BrokerTx};

struct PdfData {
    pub benefits: Vec<BenefitEntry>,
    pub trade_confs: Vec<BrokerTx>,
}

fn parse_pdfs(files: &Vec<PathBuf>, debug: bool)
-> Result<PdfData, SError> {
    let mut benefits: Vec<BenefitEntry> = Vec::new();
    let mut trade_confs: Vec<BrokerTx> = Vec::new();

    for (i, fpath) in files.iter().enumerate() {
        if i != 0 {
            if debug {
                // Line separator between entries
                eprintln!()
            }
        }
        if debug {
            eprintln!("Parsing {fpath:?}");
        }

        let pdf_text = pdf::get_all_pages_text_from_path(fpath)
            .map_err(|e| format!("Failed to read {fpath:?}: {e}"))?
            .join("\n");
        let pdf_content = etrade::parse_pdf_text(&pdf_text, fpath)
            .map_err(|e| format!("Failed to parse {fpath:?}: {e}"))?;
        match pdf_content {
            etrade::EtradePdfContent::BenefitConfirmation(mut bs) => {
                if debug {
                    eprintln!("{bs:#?}");
                }
                benefits.append(&mut bs);
            },
            etrade::EtradePdfContent::TradeConfirmation(mut txs) => {
                if debug {
                    for tx in &txs {
                        eprintln!("{tx:#?}");
                    }
                }
                trade_confs.append(&mut txs);
            },
        }
    }

    Ok(PdfData{benefits, trade_confs})
}

fn dump_extracted_data(pdf_data: &PdfData, pretty: bool) {
    let mut printer: Box<dyn AcbWriter> = if pretty {
        Box::new(TextWriter::new(WriteHandle::stdout_write_handle()))
    } else {
        Box::new(CsvWriter::new_to_writer(WriteHandle::stdout_write_handle()))
    };

    if pdf_data.benefits.is_empty() {
        eprintln!("WARN: No benefits entries");
    }
    let mut rt = RenderTable::default();
    rt.header.extend(vec![
        "security", "acquire_tx_date", "acquire_settle_date",
        "acquire_share_price", "acquire_shares",
        "sell_to_cover_tx_date", "sell_to_cover_settle_date",
        "sell_to_cover_price", "sell_to_cover_shares", "sell_to_cover_fee",
        "plan_note", "sell_note", "filename"].into_iter().map(String::from));

    for b in &pdf_data.benefits {
        // println!("{benefit:?}");
        rt.rows.push(vec![
            b.security.clone(),
            b.acquire_tx_date.to_string(),
            b.acquire_settle_date.to_string(),
            b.acquire_share_price.to_string(),
            b.acquire_shares.to_string(),

            format!("{:?}", b.sell_to_cover_tx_date),
            format!("{:?}", b.sell_to_cover_settle_date),
            format!("{:?}", b.sell_to_cover_price),
            format!("{:?}", b.sell_to_cover_shares),
            format!("{:?}", b.sell_to_cover_fee),

            b.plan_note.clone(),
            format!("{:?}", b.sell_note),
            b.filename.clone(),
        ]);
    }
    let _ = printer.print_render_table(OutputType::Raw, "benefits", &rt).unwrap();

    println!("");
    if pdf_data.trade_confs.is_empty() {
        eprintln!("WARN: No trades entries");
    }

    let mut rt = RenderTable::default();
    rt.header.extend(vec![
        "security", "trade_date", "settlement_date", "action", "amount_per_share",
        "num_shares", "commission", "currency", "memo", "exchange_rate", "affiliate",
        "row_num", "account", "sort_tiebreak", "filename"
        ].into_iter().map(String::from));

    for t in &pdf_data.trade_confs {
        // println!("{trade:?}");
        rt.rows.push(vec![
            t.security.clone(),
            format!("{} ({})", t.trade_date, t.trade_date_and_time),
            format!("{} ({})", t.settlement_date, t.settlement_date_and_time),
            t.action.to_string(),
            t.amount_per_share.to_string(),
            t.num_shares.to_string(),
            t.commission.to_string(),
            t.currency.to_string(),
            t.memo.clone(),
            format!("{:?}", t.exchange_rate),
            t.affiliate.name().to_string(),

            t.row_num.to_string(),
            t.account.memo_str(),
            format!("{:?}", t.sort_tiebreak),

            format!("{:?}", t.filename),
        ]);
    }

    let _ = printer.print_render_table(OutputType::Raw, "trades", &rt).unwrap();
}

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

    let pdf_data = parse_pdfs(&args.files, args.debug)
        .map_err(|e| eprintln!("{}", e))?;

    if args.extract_only {
        dump_extracted_data(&pdf_data, args.pretty);
    }

    Ok(())
}