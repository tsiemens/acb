use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Parser;
use itertools::Itertools;
use rust_decimal::Decimal;

use super::broker::{etrade::BenefitEntry, BrokerTx};
use crate::app::outfmt::csv::CsvWriter;
use crate::app::outfmt::model::{AcbWriter, OutputType};
use crate::app::outfmt::text::TextWriter;
use crate::peripheral::broker::etrade;
use crate::portfolio::render::RenderTable;
use crate::portfolio::{CsvTx, Currency, TxAction};
use crate::util::rw::WriteHandle;
use crate::write_errln;
use crate::{peripheral::pdf, util::basic::SError};

struct PdfData {
    pub benefits: Vec<BenefitEntry>,
    pub trade_confs: Vec<BrokerTx>,
}

fn parse_pdfs(files: &Vec<PathBuf>, debug: bool) -> Result<PdfData, SError> {
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

    Ok(PdfData {
        benefits,
        trade_confs,
    })
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

/// Constructs/extracts CsvTxs from the BenefitsAndTrades, and emits a sorted result.
/// Errors if sell-to-cover data is incomplete.
fn txs_from_data(
    trade_data: &BenefitsAndTrades,
) -> Result<Vec<crate::portfolio::CsvTx>, SError> {
    let mut csv_txs = Vec::new();

    for (i, b) in trade_data.benefits.iter().enumerate() {
        let buy_tx = crate::portfolio::CsvTx {
            security: Some(b.security.clone()),
            trade_date: Some(b.acquire_tx_date),
            settlement_date: Some(b.acquire_settle_date),
            action: Some(TxAction::Buy),
            shares: Some(b.acquire_shares),
            amount_per_share: Some(b.acquire_share_price),
            commission: Some(Decimal::ZERO),
            tx_currency: Some(Currency::usd()),
            tx_curr_to_local_exchange_rate: None,
            commission_currency: None,
            commission_curr_to_local_exchange_rate: None,
            memo: Some(b.plan_note.clone()),
            affiliate: None,
            specified_superficial_loss: None,
            stock_split_ratio: None,
            read_index: (i * 2).try_into().unwrap(),
        };

        csv_txs.push(buy_tx);

        if let Some(sell_to_cover) = b.sell_to_cover_data()? {
            let sell_note =
                b.sell_note.as_ref().map(|n| n.as_str()).unwrap_or("sell-to-cover");
            let sell_tx = CsvTx {
                security: Some(b.security.clone()),
                trade_date: Some(sell_to_cover.sell_to_cover_tx_date),
                settlement_date: Some(sell_to_cover.sell_to_cover_settle_date),
                action: Some(TxAction::Sell),
                shares: Some(sell_to_cover.sell_to_cover_shares),
                amount_per_share: Some(sell_to_cover.sell_to_cover_shares),
                commission: Some(sell_to_cover.sell_to_cover_fee),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: None,
                commission_currency: None,
                commission_curr_to_local_exchange_rate: None,
                memo: Some(format!("{} {}", b.plan_note, sell_note)),
                affiliate: None,
                specified_superficial_loss: None,
                stock_split_ratio: None,
                read_index: ((i * 2) + 1).try_into().unwrap(),
            };

            csv_txs.push(sell_tx);
        }
    }

    // Remaining trades
    let read_index_base = csv_txs.len();
    for (i, trade) in trade_data.other_trades.iter().enumerate() {
        let mut tx: CsvTx = trade.clone().into();
        tx.memo = Some(
            match tx.memo {
                Some(memo) => {
                    if memo.is_empty() {
                        memo
                    } else {
                        memo + " "
                    }
                }
                None => String::new(),
            } + "(manual trade)",
        );
        tx.read_index = (read_index_base + i).try_into().unwrap();
        csv_txs.push(tx);
    }

    csv_txs.sort();

    Ok(csv_txs)
}

fn render_txs_from_data(
    trade_data: &BenefitsAndTrades,
    pretty: bool,
    out_w: WriteHandle,
) -> Result<(), SError> {
    let txs = txs_from_data(trade_data)?;

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

/// Searches through trade_confs and finds a set of trades which fully correspond
/// to the sell-to-cover of the benefit.
///
/// trade_confs should be pre-filtered to only contain trades within a reasonable
/// date range of benefit.
fn find_sell_to_cover_trade_set<'a>(
    benefit: &BenefitEntry,
    trade_confs: &Vec<&'a BrokerTx>,
) -> Result<Vec<&'a BrokerTx>, SError> {
    let sell_to_cover_shares = benefit.sell_to_cover_shares.unwrap();

    let benefit_err_desc = || {
        format!(
            "{}: {} {}",
            benefit.filename, benefit.plan_note, benefit.acquire_tx_date
        )
    };

    // Step 1: run through combinations of trades, and find some combo where
    // the security of all matches and the number of shares matches.
    let mut all_matching_trades: Vec<Vec<&BrokerTx>> = Vec::new();
    for n in (1..=trade_confs.len()).rev() {
        tracing::trace!("find_sell_to_cover_trade_set combos len {n}");

        // (combinations from itertools crate, here)
        for trades in trade_confs.iter().combinations(n) {
            if !trades.iter().all(|t| t.security == benefit.security) {
                tracing::trace!(
                    "find_sell_to_cover_trade_set skipping set with not \
                                all securities matched"
                );
                continue;
            }
            let n_shares: Decimal = trades.iter().map(|t| t.num_shares).sum();
            if n_shares == sell_to_cover_shares {
                all_matching_trades.push(trades.into_iter().map(|t| *t).collect());
            }
        }
    }

    // Step 2: Select best trade combo set.
    if all_matching_trades.len() == 1 {
        let matching_trades = all_matching_trades.into_iter().next().unwrap();
        tracing::debug!(
            "Found matching trade confirmations for benefit:\n{:#?}",
            benefit
        );
        for t in &matching_trades {
            tracing::debug!("  {t:#?}");
        }
        Ok(matching_trades)
    } else if all_matching_trades.len() > 1 {
        tracing::trace!(
            "find_sell_to_cover_trade_set {} candidates found",
            all_matching_trades.len()
        );
        // Try to chose best trade combination match.
        // If we have two trade combos, compute the average sale price, and compute
        // which more closely approximates the price-per-share in the benefit entry.
        // If that's None, then we just fail for now.
        struct TradesCombination<'a> {
            trades: Vec<&'a BrokerTx>,
            average_price: Decimal,
            // This will be used to sort
            abs_difference_from_benefit_price: Decimal,
        }

        let mut trade_combos: Vec<TradesCombination> = all_matching_trades
            .into_iter()
            .map(|trades| {
                let total_val: Decimal =
                    trades.iter().map(|t| t.amount_per_share * t.num_shares).sum();
                let total_shares: Decimal =
                    trades.iter().map(|t| t.num_shares).sum();
                let avg_price = total_val / total_shares;
                let diff = match benefit.sell_to_cover_price {
                    Some(p) => (p - avg_price).abs(),
                    None => Decimal::MAX,
                };

                TradesCombination {
                    trades,
                    average_price: avg_price,
                    abs_difference_from_benefit_price: diff,
                }
            })
            .collect();

        // We want the closest to the benefit price first.
        trade_combos.sort_by(|a, b| {
            a.abs_difference_from_benefit_price
                .cmp(&b.abs_difference_from_benefit_price)
        });

        let combos_str = trade_combos
            .iter()
            .map(|trades| {
                trades
                    .trades
                    .iter()
                    .map(|t| format!("x {} @ {}", t.num_shares, t.amount_per_share))
                    .join(", ")
                    + format!(" (avg price {:.4})", trades.average_price).as_str()
            })
            .join("\n  ");
        tracing::debug!(
            "Average reported sale price: {:?}\n  {}",
            benefit.sell_to_cover_price,
            combos_str
        );

        // Pick the first (best) combo, provided it was actually quantafiably good.
        // If there was no sell-to-cover price, then we don't have a very good guess.
        let selected_combo_ref = &trade_combos[0];
        if selected_combo_ref.abs_difference_from_benefit_price != Decimal::MAX {
            Ok(trade_combos.into_iter().next().unwrap().trades)
        } else {
            Err(format!(
                "Unable to decide between multiple trade combinations \
                                could potentially constitute the \
                                sell-to-cover for {}:\n \
                                Average reported sale price: {:?}\n  {}",
                benefit_err_desc(),
                benefit.sell_to_cover_price,
                combos_str
            ))
        }
    } else {
        Err(format!(
            "Found no trades matching the sell-to-cover for {}",
            benefit_err_desc()
        ))
    }
}

#[derive(Debug)]
struct BenefitsAndTrades {
    pub benefits: Vec<BenefitEntry>,
    pub other_trades: Vec<BrokerTx>,
}

#[derive(Debug)]
struct AmendBenefitsRes {
    benefits_and_trades: BenefitsAndTrades,
    warnings: Vec<String>,
}

/// Goes through benefits, and populates their sell-to-cover information, based on
/// the available trade_confs.
/// Entries in trade_confs are "consumed" when this match occurs.
/// A new BenefitsAndTrades is returned.
/// Consumes the pdf_data, as it moves much of its contents into the output.
fn amend_benefit_sales(pdf_data: PdfData) -> Result<AmendBenefitsRes, Vec<SError>> {
    let trade_confs = pdf_data.trade_confs;
    let mut benefits = pdf_data.benefits;
    let mut leftover_trade_confs = trade_confs.clone();

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    for benefit in &mut benefits {
        if benefit.sell_to_cover_shares.is_none() {
            continue;
        }

        // Find the sale(s) which could constitute this sell-to-cover
        // We'll take any sell that is between the benefit data and 5 days after.
        let latest_day =
            benefit.acquire_tx_date.saturating_add(time::Duration::days(5));
        let mut candidate_trades = Vec::new();
        for trade in &leftover_trade_confs {
            if trade.action == TxAction::Sell
                && benefit.acquire_tx_date <= trade.trade_date
                && trade.trade_date <= latest_day
            {
                candidate_trades.push(trade);
            }
        }

        match find_sell_to_cover_trade_set(benefit, &candidate_trades) {
            Ok(matched_trades) => {
                // Ament the benefit dates
                let t0 = matched_trades[0];
                for t in &matched_trades {
                    if t0.trade_date != t.trade_date
                        || t0.settlement_date != t.settlement_date
                    {
                        let mut warn =
                            format!("sell-to-cover trades have varrying dates:");
                        for t_ in &matched_trades {
                            warn += &format!(
                                "\n  TD: {}, SD: {}, shares of {}: {}",
                                t_.trade_date,
                                t_.settlement_date,
                                t_.security,
                                t_.num_shares
                            );
                        }
                        warnings.push(warn);
                    }
                }
                benefit.sell_to_cover_tx_date = Some(t0.trade_date);
                benefit.sell_to_cover_settle_date = Some(t0.settlement_date);

                // Remove matches from leftover trades
                let mut indexes = Vec::<usize>::with_capacity(matched_trades.len());
                for t in matched_trades {
                    let index =
                        leftover_trade_confs.iter().position(|t_| t_ == t).unwrap();
                    indexes.push(index);
                }
                // Sort reversed
                indexes.sort();
                for i in indexes.iter().rev() {
                    leftover_trade_confs.remove(*i);
                }
            }
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        let bat = BenefitsAndTrades {
            benefits,
            other_trades: leftover_trade_confs,
        };
        Ok(AmendBenefitsRes {
            benefits_and_trades: bat,
            warnings,
        })
    } else {
        Err(errors)
    }
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
pub struct Args {
    /// ETRADE statement PDFs
    ///
    /// These can also be plain .txt files, and will not be interpreted as actual
    /// PDFs, but just the text emitted by a tool like pdf-text.
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
    if args.debug {
        crate::tracing::enable_trace_env(
            "acb::peripheral::etrade_plan_pdf_tx_extract_impl=debug",
        );
    }
    crate::tracing::setup_tracing();

    // Sort the files, so that we can deterministically output them in the same
    // order. This affects tie-breaks when we have multiple TXs on the same day.
    args.files.sort();

    let pdf_data = parse_pdfs(&args.files, args.debug)
        .map_err(|e| write_errln!(err_w, "{}", e))?;

    if args.extract_only {
        dump_extracted_data(&pdf_data, args.pretty, out_w, err_w);
        return Ok(());
    }

    let amend_res = amend_benefit_sales(pdf_data);
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

    render_txs_from_data(&benefits_and_trades, args.pretty, out_w)
        .map_err(|e| write_errln!(err_w, "Error: {e}"))?;

    Ok(())
}

// MARK: tests

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::peripheral::broker::{etrade::BenefitEntry, BrokerTx};
    use crate::peripheral::etrade_plan_pdf_tx_extract_impl::{
        amend_benefit_sales, PdfData,
    };
    use crate::portfolio::testlib::MAGIC_DEFAULT_DATE;
    use crate::portfolio::{CsvTx, Currency, TxAction};
    use crate::testlib::{assert_re, assert_vec_eq};
    use crate::util::date::pub_testlib::doy_date;

    use super::find_sell_to_cover_trade_set;

    fn foo() -> String {
        String::from("FOO")
    }

    fn dt(doy: i64) -> time::Date {
        doy_date(2024, doy)
    }

    /// Test Benefit factory
    struct TBen {
        pub sec: String,
        pub tdate: time::Date,
        pub n_sh: u32,
        pub n_stc: Option<u32>,
        pub stc_tdate: Option<time::Date>,
    }

    impl Default for TBen {
        fn default() -> Self {
            Self {
                sec: foo(),
                tdate: *MAGIC_DEFAULT_DATE,
                n_sh: 100,
                n_stc: Some(50),
                stc_tdate: None,
            }
        }
    }

    impl TBen {
        pub fn x(self) -> BenefitEntry {
            let tdate = if self.tdate == *MAGIC_DEFAULT_DATE {
                dt(10)
            } else {
                self.tdate
            };
            let sdate = tdate.saturating_add(time::Duration::days(2));
            let has_stc = self.n_stc.is_some();
            BenefitEntry {
                security: self.sec,
                acquire_tx_date: tdate,
                acquire_settle_date: sdate,
                acquire_share_price: dec!(1),
                acquire_shares: Decimal::from(self.n_sh),
                // Pre-amend states for stc.
                sell_to_cover_tx_date: self.stc_tdate,
                sell_to_cover_settle_date: self
                    .stc_tdate
                    .map(|d| d.saturating_add(time::Duration::days(2))),
                sell_to_cover_price: if has_stc { Some(dec!(100)) } else { None },
                sell_to_cover_shares: self.n_stc.map(|s| Decimal::from(s)),
                sell_to_cover_fee: if has_stc { Some(dec!(5.99)) } else { None },
                plan_note: "XXXX Vest".to_string(),
                sell_note: if has_stc {
                    Some("XXX STC".to_string())
                } else {
                    None
                },
                filename: "a_file.pdf".to_string(),
            }
        }
    }

    /// Test trade
    struct TTx {
        pub sec: String,
        pub tdate: time::Date,
        pub n_sh: u32,
        pub act: TxAction,
    }

    impl Default for TTx {
        fn default() -> Self {
            Self {
                sec: foo(),
                tdate: *MAGIC_DEFAULT_DATE,
                n_sh: 50,
                act: TxAction::Sell,
            }
        }
    }

    impl TTx {
        pub fn x(self) -> BrokerTx {
            let tdate = if self.tdate == *MAGIC_DEFAULT_DATE {
                dt(10)
            } else {
                self.tdate
            };
            let sdate = tdate.saturating_add(time::Duration::days(2));

            BrokerTx {
                security: self.sec,
                trade_date: tdate,
                settlement_date: sdate,
                trade_date_and_time: tdate.to_string(),
                settlement_date_and_time: sdate.to_string(),
                action: self.act,
                amount_per_share: dec!(1),
                num_shares: Decimal::from(self.n_sh),
                commission: dec!(5.99),
                currency: Currency::usd(),
                memo: "test trade conf".to_string(),
                exchange_rate: None,
                affiliate: crate::portfolio::Affiliate::default(),
                row_num: 100 + self.n_sh,
                account: crate::peripheral::broker::etrade::new_account(
                    "x".to_string(),
                ),
                sort_tiebreak: None,
                filename: Some("conf.pdf".to_string()),
            }
        }
    }

    fn benefit_n_shares_stc_price(
        n_shares: u32,
        stc_price: Option<Decimal>,
    ) -> BenefitEntry {
        BenefitEntry {
            security: foo(),
            acquire_tx_date: dt(10),
            acquire_settle_date: dt(10),
            acquire_share_price: dec!(1),
            acquire_shares: dec!(100),
            sell_to_cover_tx_date: Some(dt(10)),
            sell_to_cover_settle_date: Some(dt(12)),
            sell_to_cover_price: stc_price,
            sell_to_cover_shares: Some(Decimal::from(n_shares)),
            sell_to_cover_fee: Some(dec!(0)),
            plan_note: "XXXX Vest".to_string(),
            sell_note: Some("XXX STC".to_string()),
            filename: "a_file.pdf".to_string(),
        }
    }

    fn benefit_n_shares(n_shares: u32) -> BenefitEntry {
        benefit_n_shares_stc_price(n_shares, Some(dec!(100)))
    }

    fn tx_n_shares_price(n_shares: u32, price: Decimal) -> BrokerTx {
        BrokerTx {
            security: foo(),
            trade_date: dt(10),
            settlement_date: dt(12),
            trade_date_and_time: "2024-01-10".to_string(),
            settlement_date_and_time: "2024-01-12".to_string(),
            action: TxAction::Sell,
            amount_per_share: price,
            num_shares: Decimal::from(n_shares),
            commission: dec!(0),
            currency: Currency::usd(),
            memo: "Sell to cover".to_string(),
            exchange_rate: None,
            affiliate: crate::portfolio::Affiliate::default(),
            row_num: 100 + n_shares,
            account: crate::peripheral::broker::etrade::new_account("x".to_string()),
            sort_tiebreak: None,
            filename: Some("conf.pdf".to_string()),
        }
    }

    fn tx_n_shares(n_shares: u32) -> BrokerTx {
        tx_n_shares_price(n_shares, dec!(1))
    }

    fn dflt<T: Default>() -> T {
        T::default()
    }

    fn assert_sorted_vec_eq<T>(mut left: Vec<T>, mut right: Vec<T>)
    where
        T: PartialEq + Ord + std::fmt::Debug,
    {
        left.sort();
        right.sort();
        assert_vec_eq(left, right);
    }

    #[rustfmt::skip]
    #[test]
    fn test_find_sell_to_cover_trade_set() {
        if std::env::var("VERBOSE").is_ok() {
            crate::tracing::enable_trace_env(
                "acb::peripheral::etrade_plan_pdf_tx_extract_impl=trace");
        }
        crate::tracing::setup_tracing();

        // Very basic case
        let trades = vec![tx_n_shares(5)];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares(5), &trades.iter().collect()).unwrap();

        assert_vec_eq(trades.iter().collect(), matching_trades);

        // Basic multiple case
        let trades = vec![tx_n_shares(5), tx_n_shares(1)];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares(6), &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(trades.iter().collect(), matching_trades);

        // Multiple, reconcilable case
        let trades = vec![tx_n_shares(5), tx_n_shares(2), tx_n_shares(1)];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares(6), &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(vec![&trades[0], &trades[2]], matching_trades);

        // Multiple, exact duplicates (we arbitrarily take the first encountered
        // match).
        let trades = vec![tx_n_shares(5), tx_n_shares(1), tx_n_shares(12),
                          tx_n_shares(1)];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares(6), &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(vec![&trades[0], &trades[1]], matching_trades);

        // No match
        let trades = vec![tx_n_shares(5), tx_n_shares(1)];
        let err = find_sell_to_cover_trade_set(
            &benefit_n_shares(2), &trades.iter().collect()).unwrap_err();
        assert_eq!(err,
            "Found no trades matching the sell-to-cover for a_file.pdf: \
            XXXX Vest 2024-01-11");

        // No candidates
        let trades = vec![];
        let err = find_sell_to_cover_trade_set(
            &benefit_n_shares(2), &trades.iter().collect()).unwrap_err();
        assert_eq!(err,
            "Found no trades matching the sell-to-cover for a_file.pdf: \
            XXXX Vest 2024-01-11");

        // Resolve multiple possible sell-to-cover combos (by closest avg price).
        let trades = vec![tx_n_shares_price(5, dec!(400)),
                          tx_n_shares_price(1, dec!(100)),
                          tx_n_shares_price(4, dec!(101)),
                          tx_n_shares_price(2, dec!(99))];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares_stc_price(6, Some(dec!(100))),
            &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(vec![&trades[2], &trades[3]], matching_trades);

        // Resolve multiple by absolute value difference
        let trades = vec![tx_n_shares_price(6, dec!(101)),
                          tx_n_shares_price(6, dec!(50))];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares_stc_price(6, Some(dec!(100))),
            &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(vec![&trades[0]], matching_trades);

        let trades = vec![tx_n_shares_price(6, dec!(150)),
                          tx_n_shares_price(6, dec!(99))];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares_stc_price(6, Some(dec!(100))),
            &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(vec![&trades[1]], matching_trades);

        // Resolved multiples again, but where there is a preferrable combo that
        // is multiple sells, over a single sell of the exact number of shares, due
        // to price proximity.
        let trades = vec![tx_n_shares_price(5, dec!(400)),
                          tx_n_shares_price(1, dec!(99)),
                          tx_n_shares_price(4, dec!(101))];
        let matching_trades = find_sell_to_cover_trade_set(
            &benefit_n_shares_stc_price(5, Some(dec!(100))),
            &trades.iter().collect()).unwrap();
        assert_sorted_vec_eq(vec![&trades[1], &trades[2]], matching_trades);

        // Unreconcilable case (no aggregate sell-to-cover price).
        // Realistically, this case should be impossible to hit, but we cover it
        // just because the Benefit struct supports Option of the price.
        let err = find_sell_to_cover_trade_set(
            &benefit_n_shares_stc_price(5, None),
            &trades.iter().collect()).unwrap_err();
        assert_re(
            "Unable to decide between multiple trade combinations could potentially \
            constitute the sell-to-cover for a_file.pdf: XXXX Vest 2024-01-11",
            &err);
    }

    #[rustfmt::skip]
    #[test]
    fn test_amend_benefit_sales() {
        // Cases:
        // - With non-sell-to-cover benefit (and no available trades for it)
        // - Benefit with trades exactly 5 days after (and 6 days after, and before,
        //     creating error)
        // - Multiple trades for stc with inconsistent dates.
        // - Leftover trades
        // - Removing possible trades from other later stcs (would error otherwise)
        // - Ignore buys (that would break the search algo)

        // Case: With non-sell-to-cover benefit (and no available trades for it)
        let benefits = vec![TBen{tdate: dt(20), n_sh: 2, n_stc: None, ..dflt()}.x()];
        let trade_confs = vec![TTx{tdate: dt(20), n_sh: 1, ..dflt()}.x()];
        let amend_res = amend_benefit_sales(PdfData{benefits, trade_confs}).unwrap();
        assert_eq!(amend_res.benefits_and_trades.benefits[0].sell_to_cover_tx_date,
                   None);
        assert_eq!(amend_res.benefits_and_trades.other_trades.len(), 1);
        assert!(amend_res.warnings.is_empty());

        // Case: Benefit with trades exactly 5 days after
        let benefits = vec![TBen{tdate: dt(20), n_stc: Some(5), ..dflt()}.x()];
        let trade_confs = vec![TTx{tdate: dt(25), n_sh: 5, ..dflt()}.x()];
        let amend_res = amend_benefit_sales(PdfData{benefits, trade_confs}).unwrap();
        let amended_benefits = amend_res.benefits_and_trades.benefits;
        assert_eq!(amended_benefits[0].sell_to_cover_tx_date, Some(dt(25)));
        assert_eq!(amended_benefits[0].sell_to_cover_settle_date, Some(dt(27)));
        assert_eq!(amend_res.benefits_and_trades.other_trades.len(), 0);
        assert!(amend_res.warnings.is_empty());

        // Case: Benefit with trades exactly 6 days after, creating error
        let benefits = vec![TBen{tdate: dt(20), n_stc: Some(5), ..dflt()}.x()];
        let trade_confs = vec![TTx{tdate: dt(26), n_sh: 5, ..dflt()}.x()];
        let errs = amend_benefit_sales(PdfData{benefits, trade_confs})
            .unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_re("Found no trades matching the sell-to-cover for", &errs[0]);

        // Case: Benefit with trade 1 day before, creating error
        let benefits = vec![TBen{tdate: dt(20), n_stc: Some(5), ..dflt()}.x()];
        let trade_confs = vec![TTx{tdate: dt(19), n_sh: 5, ..dflt()}.x()];
        let errs = amend_benefit_sales(PdfData{benefits, trade_confs})
            .unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_re("Found no trades matching the sell-to-cover for", &errs[0]);

        // Case: Multiple trades for stc with inconsistent dates (also, leftovers)
        let benefits = vec![TBen{tdate: dt(20), n_stc: Some(5), ..dflt()}.x()];
        let trade_confs = vec![
            TTx{tdate: dt(20), n_sh: 4, ..dflt()}.x(),
            TTx{tdate: dt(21), n_sh: 1, ..dflt()}.x(),
            TTx{tdate: dt(21), n_sh: 3, ..dflt()}.x(),
            TTx{tdate: dt(1), n_sh: 3, ..dflt()}.x(),
        ];
        let amend_res = amend_benefit_sales(PdfData{benefits, trade_confs}).unwrap();
        let amended_benefits = amend_res.benefits_and_trades.benefits;
        if amended_benefits[0].sell_to_cover_tx_date != Some(dt(20)) &&
           amended_benefits[0].sell_to_cover_tx_date != Some(dt(21)) {
            // It could be either. Just pick one arbitrarily to assert on.
            assert_eq!(amended_benefits[0].sell_to_cover_tx_date, Some(dt(20)));
        }
        assert_vec_eq(amend_res.benefits_and_trades.other_trades, vec![
            TTx{tdate: dt(21), n_sh: 3, ..dflt()}.x(),
            TTx{tdate: dt(1), n_sh: 3, ..dflt()}.x(),
        ]);
        assert_eq!(amend_res.warnings.len(), 1);
        assert_re("sell-to-cover trades have varrying dates",
                  &amend_res.warnings[0]);

        // Case: Removing possible trades from other later stcs (would error
        // otherwise)
        let mut benefits = vec![
            TBen{tdate: dt(20), n_stc: Some(3), ..dflt()}.x(),
            TBen{tdate: dt(21), n_stc: Some(7), ..dflt()}.x(),
        ];
        let trade_confs = vec![
            TTx{tdate: dt(21), n_sh: 3, ..dflt()}.x(),
            TTx{tdate: dt(22), n_sh: 4, ..dflt()}.x(), // leftover
            TTx{tdate: dt(23), n_sh: 5, ..dflt()}.x(),
            TTx{tdate: dt(23), n_sh: 2, ..dflt()}.x(),
        ];
        let amend_res = amend_benefit_sales(
            PdfData{benefits: benefits.clone(), trade_confs: trade_confs.clone()})
            .unwrap();
        let amended_benefits = amend_res.benefits_and_trades.benefits;
        assert_eq!(amend_res.warnings.len(), 0);
        assert_eq!(amended_benefits[0].sell_to_cover_tx_date, Some(dt(21)));
        assert_eq!(amended_benefits[1].sell_to_cover_tx_date, Some(dt(23)));

        benefits.reverse();
        let errs = amend_benefit_sales(PdfData{benefits: benefits, trade_confs})
            .unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_re("Found no trades matching", &errs[0]);

        // Case: Ignore buys
        let benefits = vec![
            TBen{tdate: dt(20), n_stc: Some(3), ..dflt()}.x(),
        ];
        let trade_confs = vec![
            TTx{tdate: dt(21), n_sh: 3, ..dflt()}.x(),
            TTx{tdate: dt(22), n_sh: 2, act: TxAction::Buy,
                ..dflt()}.x(),
            TTx{tdate: dt(23), n_sh: 1, act: TxAction::Buy,
                ..dflt()}.x(),
        ];
        let amend_res = amend_benefit_sales(
            PdfData{benefits: benefits.clone(), trade_confs: trade_confs.clone()})
            .unwrap();
        let amended_benefits = amend_res.benefits_and_trades.benefits;
        assert_eq!(amend_res.warnings.len(), 0);
        assert_eq!(amended_benefits[0].sell_to_cover_tx_date, Some(dt(21)));
    }

    #[rustfmt::skip]
    #[test]
    fn test_txs_from_data() {

        let txs = super::txs_from_data(&super::BenefitsAndTrades {
            benefits: vec![
                // With StC
                TBen{tdate: dt(20), n_stc: Some(3), stc_tdate: Some(dt(21)),
                     ..dflt()}.x(),
                // Without StC
                TBen{tdate: dt(15), n_stc: None, ..dflt()}.x(),
            ],
            other_trades: vec![
                // Extra sell
                TTx{tdate: dt(18), n_sh: 3, ..dflt()}.x(),
                // Extra buy
                TTx{tdate: dt(19), n_sh: 2, act: TxAction::Buy,
                    ..dflt()}.x(),
            ]
        }).unwrap();

        assert_vec_eq(txs, vec![
            // Vest without Stc
            CsvTx {
                security: Some(foo()),
                trade_date: Some(dt(15)), // Some(2024-01-16),
                settlement_date: Some(dt(17)), // Some(2024-01-18),
                action: Some(TxAction::Buy),
                shares: Some(dec!(100)),
                amount_per_share: Some(dec!(1)),
                commission: Some(dec!(0)),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: None,
                commission_currency: None,
                commission_curr_to_local_exchange_rate: None,
                memo: Some("XXXX Vest".to_string()),
                affiliate: None,
                specified_superficial_loss: None,
                stock_split_ratio: None,
                read_index: 2,
            },
            // Extra sell
            CsvTx {
                security: Some(foo()),
                trade_date: Some(dt(18)), // Some(2024-01-19),
                settlement_date: Some(dt(20)), // Some(2024-01-21),
                action: Some(TxAction::Sell),
                shares: Some(dec!(3)),
                amount_per_share: Some(dec!(1)),
                commission: Some(dec!(5.99)),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: None,
                commission_currency: None,
                commission_curr_to_local_exchange_rate: None,
                memo: Some("test trade conf (manual trade)".to_string()),
                affiliate: Some(crate::portfolio::Affiliate::default()),
                specified_superficial_loss: None,
                stock_split_ratio: None,
                read_index: 3,
            },
            // Extra buy
            CsvTx {
                security: Some(foo()),
                trade_date: Some(dt(19)), // Some(2024-01-20),
                settlement_date: Some(dt(21)), // Some(2024-01-22),
                action: Some(TxAction::Buy),
                shares: Some(dec!(2)),
                amount_per_share: Some(dec!(1)),
                commission: Some(dec!(5.99)),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: None,
                commission_currency: None,
                commission_curr_to_local_exchange_rate: None,
                memo: Some("test trade conf (manual trade)".to_string()),
                affiliate: Some(crate::portfolio::Affiliate::default()),
                specified_superficial_loss: None,
                stock_split_ratio: None,
                read_index: 4 },
            // Vest with StC
            CsvTx {
                security: Some(foo()),
                trade_date: Some(dt(20)), // Some(2024-01-21),
                settlement_date: Some(dt(22)), // Some(2024-01-23),
                action: Some(TxAction::Buy),
                shares: Some(dec!(100)),
                amount_per_share: Some(dec!(1)),
                commission: Some(dec!(0)),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: None,
                commission_currency: None,
                commission_curr_to_local_exchange_rate: None,
                memo: Some("XXXX Vest".to_string()),
                affiliate: None,
                specified_superficial_loss: None,
                stock_split_ratio: None,
                read_index: 0,
            },
            // Stc
            CsvTx {
                security: Some(foo()),
                trade_date: Some(dt(21)), // Some(2024-01-22),
                settlement_date: Some(dt(23)), // Some(2024-01-24),
                action: Some(TxAction::Sell),
                shares: Some(dec!(3)),
                amount_per_share: Some(dec!(3)),
                commission: Some(dec!(5.99)),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: None,
                commission_currency: None,
                commission_curr_to_local_exchange_rate: None,
                memo: Some("XXXX Vest XXX STC".to_string()),
                affiliate: None,
                specified_superficial_loss: None,
                stock_split_ratio: None,
                read_index: 1,
            },
        ]);
    }
}
