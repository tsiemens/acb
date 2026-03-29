use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Parser;

use crate::app::outfmt::csv::CsvWriter;
use crate::app::outfmt::model::{AcbWriter, OutputType};
use crate::app::outfmt::text::TextWriter;
use crate::peripheral::broker::etrade;
use crate::peripheral::pdf;
use crate::portfolio::render::RenderTable;
use crate::util::basic::SError;
use crate::util::date::DateRange;
use crate::util::rw::WriteHandle;
use crate::write_errln;

use super::etrade_plan_pdf_tx_extract_impl::PdfData;

fn get_filename(fpath: &PathBuf) -> String {
    fpath
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "<unnamed>".to_string())
}

fn is_xlsx(fpath: &PathBuf) -> bool {
    matches!(
        fpath.extension().and_then(|e| e.to_str()),
        Some("xlsx" | "xls")
    )
}

pub(super) fn parse_files(
    files: &Vec<PathBuf>,
    date_range: Option<&DateRange>,
    debug: bool,
) -> Result<PdfData, SError> {
    let mut benefits = Vec::new();
    let mut trade_confs = Vec::new();

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

        if is_xlsx(fpath) {
            let data = std::fs::read(fpath)
                .map_err(|e| format!("Failed to read {fpath:?}: {e}"))?;
            let mut xl_benefits = etrade::parse_benefit_history_xlsx(
                data,
                &get_filename(fpath),
                date_range,
            )
            .map_err(|e| format!("Failed to parse {fpath:?}: {e}"))?;
            if debug {
                eprintln!("{xl_benefits:#?}");
            }
            benefits.append(&mut xl_benefits);
            continue;
        }

        let pdf_text = if fpath.extension().unwrap_or_default().to_string_lossy()
            == "txt"
        {
            // This is mostly for testing. We can just read the pre-parsed pdf text
            tracing::trace!("Getting raw text from {:?}", fpath);
            let mut buf = String::new();
            std::fs::File::open(fpath)
                .map_err(|e| format!("Failed to open text file {fpath:?}: {e}"))?
                .read_to_string(&mut buf)
                .map_err(|e| format!("Failed to read text file {fpath:?}: {e}"))?;
            buf
        } else {
            pdf::get_all_pages_text_from_path(fpath)
                .map_err(|e| format!("Failed to read {fpath:?}: {e}"))?
                .join("\n")
        };

        let pdf_content = etrade::parse_pdf_text(&pdf_text, fpath)
            .map_err(|e| format!("Failed to parse {fpath:?}: {e}"))?;
        match pdf_content {
            etrade::EtradePdfContent::BenefitConfirmation(mut bs) => {
                if debug {
                    eprintln!("{bs:#?}");
                }
                benefits.append(&mut bs);
            }
            etrade::EtradePdfContent::TradeConfirmation(mut txs) => {
                if debug {
                    for tx in &txs {
                        eprintln!("{tx:#?}");
                    }
                }
                trade_confs.append(&mut txs);
            }
        }
    }

    if let Some(range) = date_range {
        etrade::filter_benefits_by_date(&mut benefits, range);
        trade_confs.retain(|t| range.contains(&t.settlement_date));
    }

    Ok(PdfData {
        benefits,
        trade_confs,
    })
}

fn display_opt<T: std::fmt::Display>(val: &Option<T>) -> String {
    val.as_ref().map_or("".to_string(), |v| v.to_string())
}

fn dump_extracted_data(
    pdf_data: &PdfData,
    pretty: bool,
    mut out_w: WriteHandle,
    mut err_w: WriteHandle,
) {
    let mut printer: Box<dyn AcbWriter> = if pretty {
        Box::new(TextWriter::new(out_w.clone()))
    } else {
        Box::new(CsvWriter::new_to_writer(out_w.clone()))
    };

    if pdf_data.benefits.is_empty() {
        write_errln!(err_w, "WARN: No benefits entries");
    }
    let mut rt = RenderTable::default();
    rt.header.extend(
        vec![
            "security",
            "acquire_tx_date",
            "acquire_settle_date",
            "acquire_share_price",
            "acquire_shares",
            "sell_to_cover_tx_date",
            "sell_to_cover_settle_date",
            "sell_to_cover_price",
            "sell_to_cover_shares",
            "sell_to_cover_fee",
            "plan_note",
            "sell_note",
            "filename",
        ]
        .into_iter()
        .map(String::from),
    );

    for b in &pdf_data.benefits {
        rt.rows.push(vec![
            b.security.clone(),
            b.acquire_tx_date.to_string(),
            b.acquire_settle_date.to_string(),
            b.acquire_share_price.to_string(),
            b.acquire_shares.to_string(),
            display_opt(&b.sell_to_cover_tx_date),
            display_opt(&b.sell_to_cover_settle_date),
            display_opt(&b.sell_to_cover_price),
            display_opt(&b.sell_to_cover_shares),
            display_opt(&b.sell_to_cover_fee),
            b.plan_note.clone(),
            display_opt(&b.sell_note),
            b.filename.clone(),
        ]);
    }
    let _ = printer.print_render_table(OutputType::Raw, "benefits", &rt).unwrap();

    let _ = writeln!(out_w, "");
    if pdf_data.trade_confs.is_empty() {
        write_errln!(err_w, "WARN: No trades entries");
    }

    let mut rt = RenderTable::default();
    rt.header.extend(
        vec![
            "security",
            "trade_date",
            "settlement_date",
            "action",
            "amount_per_share",
            "num_shares",
            "commission",
            "currency",
            "memo",
            "exchange_rate",
            "affiliate",
            "row_num",
            "account",
            "sort_tiebreak",
            "filename",
        ]
        .into_iter()
        .map(String::from),
    );

    for t in &pdf_data.trade_confs {
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
            display_opt(&t.exchange_rate),
            t.affiliate.name().to_string(),
            t.row_num.to_string(),
            t.account.memo_str(),
            display_opt(&t.sort_tiebreak),
            display_opt(&t.filename),
        ]);
    }

    let _ = printer.print_render_table(OutputType::Raw, "trades", &rt).unwrap();
}

fn render_txs_from_data(
    trade_data: &super::etrade_plan_pdf_tx_extract_impl::BenefitsAndTrades,
    pretty: bool,
    generate_fx: bool,
    no_sell_to_cover_pair: bool,
    out_w: WriteHandle,
) -> Result<(), SError> {
    let txs = super::etrade_plan_pdf_tx_extract_impl::txs_from_data(
        trade_data,
        generate_fx,
        no_sell_to_cover_pair,
    )?;

    let mut printer: Box<dyn AcbWriter> = if pretty {
        Box::new(TextWriter::new(out_w))
    } else {
        Box::new(CsvWriter::new_to_writer(out_w))
    };
    let table_name = if pretty { "Benefit TXs" } else { "benefit_txs" };
    let csv_table = crate::portfolio::io::tx_csv::txs_to_csv_table(&txs);
    printer.print_render_table(
        crate::app::outfmt::model::OutputType::Raw,
        &table_name,
        &crate::portfolio::render::RenderTable::from(csv_table),
    )
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
///
/// Alternatively, you can export the BenefitHistory xlsx from E*TRADE and pass
/// it in place of the benefit confirmation PDFs. However, note that the xlsx
/// does not contain enough information to pair RSU sell-to-cover trades with
/// trade confirmations. RSU entries from xlsx will behave as if
/// --no-sell-to-cover-pair was passed (ESPP entries are unaffected).
/// For best results with RSUs, use the individual benefit confirmation PDFs.
#[derive(Parser, Debug)]
#[command(author, about, long_about)]
pub struct Args {
    /// ETRADE statement PDFs, BenefitHistory xlsx files, or plain .txt files.
    ///
    /// .xlsx files are parsed as BenefitHistory exports (ESPP/RSU benefit data).
    /// Note: xlsx does not provide RSU sell-to-cover share counts, so RSU
    /// sell-to-cover pairing will not work from xlsx. Recommended to use PDFs only.
    /// .txt files are treated as pre-extracted PDF text (for testing).
    /// All other files are treated as PDFs.
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

    /// Do not generate FX transactions for manual trades
    #[arg(long)]
    pub no_fx: bool,

    /// Do not try to pair sell-to-cover data from benefit confirmations
    /// with trade confirmations. All trade confirmations become standalone
    /// sell transactions, and benefit sell-to-cover data is ignored.
    #[arg(long)]
    pub no_sell_to_cover_pair: bool,

    /// Only include entries whose settlement date falls in this year.
    /// Useful when processing a BenefitHistory xlsx that spans many years.
    #[arg(long)]
    pub year: Option<i32>,
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();
    run_with_args(
        args,
        WriteHandle::stdout_write_handle(),
        WriteHandle::stderr_write_handle(),
    )
}

pub fn run_with_args(
    mut args: Args,
    out_w: WriteHandle,
    mut err_w: WriteHandle,
) -> Result<(), ()> {
    use super::etrade_plan_pdf_tx_extract_impl::amend_benefit_sales;

    if args.debug {
        crate::tracing::enable_trace_env(
            "acb::peripheral::etrade_plan_pdf_tx_extract_cmd=debug,\
             acb::peripheral::etrade_plan_pdf_tx_extract_impl=debug",
        );
    }
    crate::tracing::setup_tracing();

    let date_range = args.year.map(DateRange::for_year);

    // Sort the files, so that we can deterministically output them in the same
    // order. This affects tie-breaks when we have multiple TXs on the same day.
    args.files.sort();

    let pdf_data = parse_files(&args.files, date_range.as_ref(), args.debug)
        .map_err(|e| write_errln!(err_w, "{}", e))?;

    if args.extract_only {
        dump_extracted_data(&pdf_data, args.pretty, out_w, err_w);
        return Ok(());
    }

    let amend_res = amend_benefit_sales(pdf_data, args.no_sell_to_cover_pair);
    let benefits_and_trades = match amend_res {
        Ok(r) => {
            for w in r.warnings {
                write_errln!(err_w, "Warning: {w}");
            }
            r.benefits_and_trades
        }
        Err(errs) => {
            for err in errs {
                write_errln!(err_w, "Error: {err}");
            }
            return Err(());
        }
    };

    if args.debug {
        // Do not use err_w for debug.
        eprintln!("\nAmmended benefit entries:");
        for b in &benefits_and_trades.benefits {
            eprintln!("{b:#?}");
        }
        eprintln!("\nRemaining trades:");
        for t in &benefits_and_trades.other_trades {
            eprintln!("{t:#?}");
        }
    }

    render_txs_from_data(
        &benefits_and_trades,
        args.pretty,
        !args.no_fx,
        args.no_sell_to_cover_pair,
        out_w,
    )
    .map_err(|e| write_errln!(err_w, "Error: {e}"))?;

    Ok(())
}
