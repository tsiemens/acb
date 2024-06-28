use std::collections::HashMap;
use std::io::Write;

use time::Date;

use crate::{
    fx::io::RateLoader,
    portfolio::{
        bookkeeping::{txs_to_delta_list, DeltaListResult},
        calc_cumulative_capital_gains, calc_security_cumulative_capital_gains,
        io::{
            tx_csv::{parse_tx_csv, write_txs_to_csv, TxCsvParseOptions},
            tx_loader::load_tx_rates,
        },
        render::{
            render_aggregate_capital_gains, render_tx_table_model, CostsTables,
            RenderTable,
        },
        summary::{make_aggregate_summary_txs, CollectedSummaryData},
        CumulativeCapitalGains, PortfolioSecurityStatus, Security, Tx, TxDelta,
    },
    util::rw::{DescribedReader, WriteHandle},
    write_errln,
};

use super::outfmt::model::{AcbWriter, OutputType};

pub type Error = String;

pub struct Options {
    pub render_full_dollar_values: bool,
    pub summary_mode_latest_date: Option<Date>,
    pub split_annual_summary_gains: bool,
    pub render_total_costs: bool,
    pub csv_output_dir: Option<String>,
    pub csv_parse_options: TxCsvParseOptions,
}

impl Options {
    pub fn summary_mode(&self) -> bool {
        self.summary_mode_latest_date.is_some()
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            render_full_dollar_values: false,
            summary_mode_latest_date: None,
            split_annual_summary_gains: false,
            render_total_costs: false,
            csv_output_dir: None,
            csv_parse_options: TxCsvParseOptions::default(),
        }
    }
}

/// This is a partial component of the app as a whole, just to generate TxDeltas.
/// What this does _not_ do is do any aggregation calculations, like
/// yearly capital gains and costs.
pub async fn run_acb_app_to_delta_models(
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    csv_parse_options: &TxCsvParseOptions,
    mut rate_loader: RateLoader,
    mut err_printer: WriteHandle,
) -> Result<HashMap<Security, DeltaListResult>, Error> {
    let mut all_txs = Vec::<Tx>::new();
    let mut global_read_index: u32 = 0;
    for mut csv_reader in csv_file_readers {
        let mut csv_txs = parse_tx_csv(
            &mut csv_reader,
            global_read_index,
            &csv_parse_options,
            &mut err_printer,
        )?;

        load_tx_rates(&mut csv_txs, &mut rate_loader).await?;

        let mut txs = Vec::<Tx>::with_capacity(csv_txs.len());
        for csv_tx in csv_txs {
            txs.push(Tx::try_from(csv_tx)?)
        }

        global_read_index += txs.len() as u32;
        all_txs.append(&mut txs);
    }

    all_txs.sort();
    let txs_by_sec = crate::portfolio::split_txs_by_security(all_txs);

    let mut delta_results = HashMap::<Security, DeltaListResult>::new();

    for (sec, sec_txs) in txs_by_sec {
        let sec_init_status =
            all_init_status.get(&sec).map(|o| std::rc::Rc::new(o.clone()));

        let deltas_res = txs_to_delta_list(&sec_txs, sec_init_status);
        delta_results.insert(sec, deltas_res);
    }

    Ok(delta_results)
}

struct AllCumulativeCapitalGains {
    pub security_gains: HashMap<Security, CumulativeCapitalGains>,
    pub aggregate_gains: CumulativeCapitalGains,
}

fn get_cumulative_capital_gains(
    deltas_by_sec: &HashMap<Security, DeltaListResult>,
) -> AllCumulativeCapitalGains {
    let mut security_gains = HashMap::<Security, CumulativeCapitalGains>::new();
    for (sec, deltas_res) in deltas_by_sec {
        if let Ok(deltas) = &deltas_res.0 {
            security_gains
                .insert(sec.clone(), calc_security_cumulative_capital_gains(deltas));
        }
    }
    let aggregate_gains = calc_cumulative_capital_gains(&security_gains);
    AllCumulativeCapitalGains {
        security_gains,
        aggregate_gains,
    }
}

pub struct AppRenderResult {
    pub security_tables: HashMap<Security, RenderTable>,
    pub aggregate_gains_table: RenderTable,
    pub costs_tables: Option<CostsTables>,
}

/// Runs the entire ACB app in the "default" mode (processing TXs and
/// generating and rendering a delta list plus some aggregations).
/// This is output as a generic render model, so that it can be fed
/// to alternate output formatters (like to console, CSV, or javascript object).
pub async fn run_acb_app_to_render_model(
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    csv_parse_options: &TxCsvParseOptions,
    render_full_dollar_values: bool,
    render_total_costs: bool,
    rate_loader: RateLoader,
    err_printer: WriteHandle,
) -> Result<AppRenderResult, Error> {
    let deltas_results_by_sec = run_acb_app_to_delta_models(
        csv_file_readers,
        all_init_status,
        csv_parse_options,
        rate_loader,
        err_printer,
    )
    .await?;

    let gains = get_cumulative_capital_gains(&deltas_results_by_sec);

    let default_gains = CumulativeCapitalGains::default();

    let mut all_deltas = Vec::<TxDelta>::new();
    let mut sec_render_tables = HashMap::new();
    for (sec, deltas_res) in deltas_results_by_sec {
        let deltas = deltas_res.deltas_or_partial_deltas();
        let mut deltas_copy = deltas.iter().cloned().collect();
        all_deltas.append(&mut deltas_copy);
        let mut table_model = render_tx_table_model(
            deltas,
            gains.security_gains.get(&sec).unwrap_or(&default_gains),
            render_full_dollar_values,
        );
        if let Err(e) = &deltas_res.0 {
            table_model.errors.push(e.err_msg.clone());
        }
        sec_render_tables.insert(sec.clone(), table_model);
    }

    let cumulative_gains_table = render_aggregate_capital_gains(
        &gains.aggregate_gains,
        render_full_dollar_values,
    );

    let costs_tables = if render_total_costs {
        let costs = crate::portfolio::bookkeeping::calc_total_costs(&all_deltas);
        Some(crate::portfolio::render::render_total_costs(
            &costs,
            render_full_dollar_values,
        ))
    } else {
        None
    };

    Ok(AppRenderResult {
        security_tables: sec_render_tables,
        aggregate_gains_table: cumulative_gains_table,
        costs_tables: costs_tables,
    })
}

fn write_render_result(
    render_res: &AppRenderResult,
    writer: &mut dyn AcbWriter,
) -> Result<(), Error> {
    let sec_render_tables = &render_res.security_tables;

    let mut secs: Vec<Security> = sec_render_tables.keys().cloned().collect();
    secs.sort();

    let mut secs_with_errors = Vec::<Security>::new();
    for sec in &secs {
        let render_table = sec_render_tables.get(sec).unwrap();
        if let Err(err) =
            writer.print_render_table(OutputType::Transactions, sec, render_table)
        {
            return Err(format!("Rendering transactions for {sec}: {err}"));
        }
        if render_table.errors.len() > 0 {
            secs_with_errors.push(sec.clone());
        }
    }

    if let Err(err) = writer.print_render_table(
        OutputType::AggregateGains,
        "",
        &render_res.aggregate_gains_table,
    ) {
        return Err(format!("Rendering aggregate gains: {err}"));
    }

    if let Some(costs_tables) = &render_res.costs_tables {
        if let Err(err) = writer.print_render_table(
            OutputType::Costs,
            "Total",
            &costs_tables.total,
        ) {
            return Err(format!("Rendering total costs: {err}"));
        }

        if let Err(err) = writer.print_render_table(
            OutputType::Costs,
            "Yearly Max",
            &costs_tables.yearly,
        ) {
            return Err(format!("Rendering yearly costs: {err}"));
        }
    }

    if secs_with_errors.len() > 0 {
        println!(
            "\n[!] There are errors for the following securities: {}",
            secs_with_errors.join(", ")
        );
    }

    Ok(())
}

/// Returned Err is for exit code determination only.
/// All errors are written to err_printer.
pub async fn run_acb_app_to_writer(
    writer: &mut dyn AcbWriter,
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    csv_parse_options: &TxCsvParseOptions,
    render_full_dollar_values: bool,
    render_total_costs: bool,
    rate_loader: RateLoader,
    mut err_printer: WriteHandle,
) -> Result<AppRenderResult, ()> {
    let res = run_acb_app_to_render_model(
        csv_file_readers,
        all_init_status,
        csv_parse_options,
        render_full_dollar_values,
        render_total_costs,
        rate_loader,
        err_printer.clone(),
    )
    .await;

    let render_res: AppRenderResult = match res {
        Ok(render_res) => render_res,
        Err(e) => {
            write_errln!(err_printer, "{}", e);
            return Err(());
        }
    };

    if let Err(e) = write_render_result(&render_res, writer) {
        write_errln!(err_printer, "{}", e);
        return Err(());
    }

    Ok(render_res)
}

pub struct AppSummaryError {
    pub general_error: Option<Error>,
    pub sec_errors: HashMap<Security, Error>,
}

pub async fn run_acb_app_summary_to_model(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    options: Options,
    rate_loader: RateLoader,
    err_printer: WriteHandle,
) -> Result<CollectedSummaryData, AppSummaryError> {
    let deltas_results_by_sec = run_acb_app_to_delta_models(
        csv_file_readers,
        all_init_status,
        &options.csv_parse_options,
        rate_loader,
        err_printer,
    )
    .await
    .map_err(|e| AppSummaryError {
        general_error: Some(e),
        sec_errors: HashMap::new(),
    })?;

    let mut deltas_by_sec = HashMap::<Security, Vec<TxDelta>>::new();
    let mut delta_errors = HashMap::new();
    for (sec, delta_res) in deltas_results_by_sec {
        match delta_res.0 {
            Ok(deltas) => {
                deltas_by_sec.insert(sec.clone(), deltas);
            }
            Err(e) => {
                delta_errors.insert(sec.clone(), e.err_msg);
            }
        }
    }

    if delta_errors.len() > 0 {
        return Err(AppSummaryError {
            general_error: None,
            sec_errors: delta_errors,
        });
    }

    Ok(make_aggregate_summary_txs(
        latest_date,
        &deltas_by_sec,
        options.split_annual_summary_gains,
    ))
}

pub async fn run_acb_app_summary_to_console(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    options: Options,
    rate_loader: RateLoader,
    mut err_printer: WriteHandle,
) -> Result<(), ()> {
    let summ_res = run_acb_app_summary_to_model(
        latest_date,
        csv_file_readers,
        all_init_status,
        options,
        rate_loader,
        err_printer.clone(),
    )
    .await;

    let summ_data = match summ_res {
        Ok(summ_data) => summ_data,
        Err(err_struct) => {
            if let Some(e) = err_struct.general_error {
                write_errln!(err_printer, "Error: {e}");
            }
            for (sec, e) in err_struct.sec_errors {
                write_errln!(err_printer, "Error in {sec}: {e}");
            }
            return Err(());
        }
    };

    if summ_data.warnings.len() > 0 {
        write_errln!(err_printer, "Warnings:");
        for (warning, secs) in summ_data.warnings {
            write_errln!(
                err_printer,
                " {}. Encountered for {}",
                warning,
                secs.join(",")
            );
        }
        write_errln!(err_printer, "");
    }

    if summ_data.txs.len() > 0 {
        let csv_txs: Vec<crate::portfolio::CsvTx> = summ_data
            .txs
            .into_iter()
            .map(|tx| crate::portfolio::CsvTx::from(tx))
            .collect();
        match write_txs_to_csv(&csv_txs, &mut WriteHandle::stdout_write_handle()) {
            Ok(()) => (),
            Err(e) => {
                write_errln!(err_printer, "Error: {e}");
                return Err(());
            }
        }
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_acb_app_to_console(
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    options: Options,
    rate_loader: RateLoader,
    mut err_printer: WriteHandle,
) -> Result<(), ()> {
    if let Some(summary_mode_latest_date) = options.summary_mode_latest_date {
        run_acb_app_summary_to_console(
            summary_mode_latest_date,
            csv_file_readers,
            all_init_status,
            options,
            rate_loader,
            err_printer,
        )
        .await
    } else {
        let mut writer: Box<dyn AcbWriter> = match options.csv_output_dir {
            Some(dir_path) => match super::outfmt::csv::CsvWriter::new_to_output_dir(&dir_path) {
                Ok(w) => Box::new(w),
                Err(e) => {
                    write_errln!(err_printer, "{e}");
                    return Err(());
                }
            },
            None => Box::new(super::outfmt::text::TextWriter::new(
                WriteHandle::stdout_write_handle(),
            )),
        };
        let writer_ref: &mut dyn AcbWriter = writer.as_mut();

        run_acb_app_to_writer(
            writer_ref,
            csv_file_readers,
            all_init_status,
            &options.csv_parse_options,
            options.render_full_dollar_values,
            options.render_total_costs,
            rate_loader,
            err_printer,
        )
        .await
        .map(|_| ())
    }
}

// MARK: Tests
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_std::task::block_on;

    use crate::portfolio::io::tx_csv::testlib::TestTxCsvRow as Row;
    use crate::testlib::assert_re;
    use crate::{
        app::outfmt::{model::AcbWriter, text::TextWriter},
        fx::io::{InMemoryRatesCache, JsonRemoteRateLoader, RateLoader},
        portfolio::{
            io::tx_csv::{testlib::CsvFileBuilder, TxCsvParseOptions},
            render::RenderTable,
            Security,
        },
        util::{http::testlib::UnusableHttpRequester, rw::WriteHandle},
    };

    use super::run_acb_app_to_render_model;

    fn smoke_test_render(render_table: &RenderTable) {
        let wh = if std::env::var("VERBOSE").unwrap_or(String::new()).is_empty() {
            WriteHandle::empty_write_handle()
        } else {
            WriteHandle::stderr_write_handle()
        };
        let mut w = TextWriter::new(wh);
        w.print_render_table(
            crate::app::outfmt::model::OutputType::Transactions,
            "Dummy table",
            render_table,
        )
        .unwrap();
    }

    fn get_total_cap_gain(render_table: &RenderTable) -> &str {
        render_table.footer[9].split("\n").into_iter().next().unwrap()
    }

    fn get_and_check_foo_table(
        render_tables: &HashMap<Security, RenderTable>,
    ) -> &RenderTable {
        assert_eq!(render_tables.len(), 1);
        let render_table = render_tables.get("FOO").unwrap();
        smoke_test_render(render_table);
        render_table
    }

    fn make_empty_test_rate_loader() -> RateLoader {
        RateLoader::new(
            false,
            Box::new(InMemoryRatesCache::new()),
            JsonRemoteRateLoader::new_boxed(UnusableHttpRequester::new_boxed()),
            WriteHandle::empty_write_handle(),
        )
    }

    fn do_test_same_day_buy_sells(render_costs: bool, csv_splits: Vec<usize>) {
        #[rustfmt::skip]
        let readers =
            CsvFileBuilder::with_all_modern_headers()
            .split_csv_rows(&csv_splits, &vec![
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "20", aps: "1.5", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Sell", sh: "5", aps: "1.6", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "5", aps: "1.7", cur: "CAD", ..Row::default()},
            ]);

        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            HashMap::new(),
            &TxCsvParseOptions::default(),
            false,
            render_costs,
            make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();

        let render_table = get_and_check_foo_table(&render_res.security_tables);
        assert_eq!(render_table.rows.len(), 3);
        assert_eq!(Vec::<String>::new(), render_table.errors);
        assert_eq!("$0.50", get_total_cap_gain(render_table));
    }

    #[test]
    fn test_same_day_buy_sells() {
        do_test_same_day_buy_sells(false, vec![3]);
        do_test_same_day_buy_sells(false, vec![1, 2]);
        do_test_same_day_buy_sells(true, vec![1, 2]);
    }

    fn do_test_negative_stocks(render_costs: bool) {
        #[rustfmt::skip]
        let readers =
            CsvFileBuilder::with_all_modern_headers()
            .split_csv_rows(&vec![1], &vec![
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Sell", sh: "5", aps: "1.6", cur: "CAD", ..Row::default()},
            ]);

        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            HashMap::new(),
            &TxCsvParseOptions::default(),
            false,
            render_costs,
            make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();

        let render_table = get_and_check_foo_table(&render_res.security_tables);
        assert_eq!(render_table.rows.len(), 0);
        assert_re(
            "is more than the current holdings",
            render_table.errors[0].as_str(),
        );
        assert_eq!("$0.00", get_total_cap_gain(render_table));
    }

    #[test]
    fn test_negative_stocks() {
        do_test_negative_stocks(false);
        do_test_negative_stocks(true);
    }

    fn do_test_fractional_shares(render_costs: bool) {
        #[rustfmt::skip]
        let readers =
            CsvFileBuilder::with_all_modern_headers()
            .split_csv_rows(&vec![3], &vec![
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "0.1", aps: "1.6", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Sell", sh: "0.05", aps: "1.7", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-04", sd: "2016-01-06",
                    a: "Sell", sh: "0.05", aps: "1.7", cur: "CAD", ..Row::default()},
            ]);

        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            HashMap::new(),
            &TxCsvParseOptions::default(),
            false,
            render_costs,
            make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();

        let render_table = get_and_check_foo_table(&render_res.security_tables);
        assert_eq!(render_table.rows.len(), 3);
        assert_eq!(Vec::<String>::new(), render_table.errors);
        assert_eq!("$0.01", get_total_cap_gain(render_table));
    }

    #[test]
    fn test_fractional_shares() {
        do_test_fractional_shares(false);
        do_test_fractional_shares(true);
    }
}
