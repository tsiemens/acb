use std::collections::HashMap;
use std::io::Write;

use crate::{
    app::approot::approot_common::{self, Error},
    fx::io::RateLoader,
    portfolio::{
        bookkeeping::DeltaListResult,
        calc_cumulative_capital_gains, calc_security_cumulative_capital_gains,
        io::tx_csv::TxCsvParseOptions,
        render::{
            render_aggregate_capital_gains, render_tx_table_model, CostsTables,
            RenderTable,
        },
        AffiliateFilter, CumulativeCapitalGains, Security, TxDelta,
    },
    util::rw::{DescribedReader, WriteHandle},
    write_errln,
};

use crate::app::outfmt::model::AcbWriter;

pub struct AppRenderResult {
    pub security_tables: HashMap<Security, RenderTable>,
    pub aggregate_gains_table: RenderTable,
    pub costs_tables: Option<CostsTables>,
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

fn filter_deltas_by_affiliate(
    deltas_by_sec: HashMap<Security, DeltaListResult>,
    affiliate_filter: &AffiliateFilter,
) -> HashMap<Security, DeltaListResult> {
    let mut filtered = HashMap::<Security, DeltaListResult>::new();
    for (sec, deltas_res) in deltas_by_sec {
        let filtered_deltas_res = match deltas_res.0 {
            Ok(deltas) => {
                let filtered_deltas: Vec<TxDelta> = deltas
                    .into_iter()
                    .filter(|delta| affiliate_filter.matches(&delta.tx.affiliate))
                    .collect();
                if filtered_deltas.len() == 0 {
                    continue;
                }
                DeltaListResult(Ok(filtered_deltas))
            }
            Err(e) => DeltaListResult(Err(e)),
        };
        filtered.insert(sec.clone(), filtered_deltas_res);
    }
    filtered
}

/// Runs the entire ACB app in the "default" mode (processing TXs and
/// generating and rendering a delta list plus some aggregations).
/// This is output as a generic render model, so that it can be fed
/// to alternate output formatters (like to console, CSV, or javascript object).
pub async fn run_acb_app_to_render_model(
    csv_file_readers: Vec<DescribedReader>,
    csv_parse_options: &TxCsvParseOptions,
    affiliate_render_filter: Option<AffiliateFilter>,
    render_full_dollar_values: bool,
    render_total_costs: bool,
    rate_loader: &mut RateLoader,
    err_printer: WriteHandle,
) -> Result<AppRenderResult, Error> {
    let deltas_results_by_sec = approot_common::run_acb_app_to_delta_models(
        csv_file_readers,
        csv_parse_options,
        rate_loader,
        err_printer,
    )
    .await?;

    let filtered_deltas_results_by_sec = match affiliate_render_filter {
        None => deltas_results_by_sec,
        Some(filter) => filter_deltas_by_affiliate(deltas_results_by_sec, &filter),
    };

    let gains = get_cumulative_capital_gains(&filtered_deltas_results_by_sec);

    let default_gains = CumulativeCapitalGains::default();

    let mut all_deltas = Vec::<TxDelta>::new();
    let mut sec_render_tables = HashMap::new();
    for (sec, deltas_res) in filtered_deltas_results_by_sec {
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
    mut writer: Box<dyn AcbWriter>,
) -> Result<(), Error> {
    let sec_render_tables = &render_res.security_tables;

    let mut secs: Vec<Security> = sec_render_tables.keys().cloned().collect();
    secs.sort();

    let mut secs_with_errors = Vec::<Security>::new();
    for sec in &secs {
        let render_table = sec_render_tables.get(sec).unwrap();
        if let Err(err) = writer.print_render_table(
            &format!("Transactions for {sec}"),
            &format!("{sec}.csv"),
            render_table,
        ) {
            return Err(format!("Rendering transactions for {sec}: {err}"));
        }
        if render_table.errors.len() > 0 {
            secs_with_errors.push(sec.clone());
        }
    }

    if let Err(err) = writer.print_render_table(
        "Aggregate Gains",
        "aggregate-gains.csv",
        &render_res.aggregate_gains_table,
    ) {
        return Err(format!("Rendering aggregate gains: {err}"));
    }

    if let Some(costs_tables) = &render_res.costs_tables {
        if let Err(err) = writer.print_render_table(
            "Total Costs",
            "total-costs.csv",
            &costs_tables.total,
        ) {
            return Err(format!("Rendering total costs: {err}"));
        }

        if let Err(err) = writer.print_render_table(
            "Yearly Max Costs",
            "yearly-max-costs.csv",
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

    writer.finish()
}

/// Returned Err is for exit code determination only.
/// All errors are written to err_printer.
pub async fn run_acb_app_to_writer(
    writer: Box<dyn AcbWriter>,
    csv_file_readers: Vec<DescribedReader>,
    csv_parse_options: &TxCsvParseOptions,
    affiliate_render_filter: Option<AffiliateFilter>,
    render_full_dollar_values: bool,
    render_total_costs: bool,
    rate_loader: &mut RateLoader,
    mut err_printer: WriteHandle,
) -> Result<AppRenderResult, ()> {
    let res = run_acb_app_to_render_model(
        csv_file_readers,
        csv_parse_options,
        affiliate_render_filter,
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

// MARK: Tests
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_std::task::block_on;

    use crate::portfolio::io::tx_csv::testlib::TestTxCsvRow as Row;
    use crate::portfolio::AffiliateFilter;
    use crate::testlib::{assert_re, assert_vec_eq};
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
            "Transactions for Dummy table",
            "Dummy table.csv",
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
                // Secondary check: displayed gain on this should round up.
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Sell", sh: "5", aps: "1.599", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "5", aps: "1.7", cur: "CAD", ..Row::default()},
            ]);

        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            &TxCsvParseOptions::default(),
            None,
            false,
            render_costs,
            &mut make_empty_test_rate_loader(),
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
            &TxCsvParseOptions::default(),
            None,
            false,
            render_costs,
            &mut make_empty_test_rate_loader(),
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
            &TxCsvParseOptions::default(),
            None,
            false,
            render_costs,
            &mut make_empty_test_rate_loader(),
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

    #[test]
    fn test_multi_af_filtering() {
        // Tests:
        // - Shared symbol between AFs: both affiliates' rows appear with no filter
        // - Symbol only in one AF: omitted when the other AF is filtered
        // - Filtering reduces rows to only the matching affiliate

        #[rustfmt::skip]
        let make_readers = || {
            CsvFileBuilder::with_all_modern_headers()
            .split_csv_rows(&vec![4], &vec![
                // FOO appears in both affiliates
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "10", aps: "1.0", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-04", sd: "2016-01-06",
                    a: "Buy", sh: "5", aps: "2.0", cur: "CAD", af: "spouse", ..Row::default()},
                // BAR only in spouse
                Row{sec: "BAR", td: "2016-02-01", sd: "2016-02-03",
                    a: "Buy", sh: "3", aps: "1.0", cur: "CAD", af: "spouse", ..Row::default()},
                // BAZ only in default
                Row{sec: "BAZ", td: "2016-03-01", sd: "2016-03-03",
                    a: "Buy", sh: "4", aps: "1.5", cur: "CAD", ..Row::default()},
            ])
        };

        // No filter: all three securities visible
        let render_res = block_on(run_acb_app_to_render_model(
            make_readers(),
            &TxCsvParseOptions::default(),
            None,
            false,
            false,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();
        assert_eq!(render_res.security_tables.len(), 3);
        // FOO has rows from both affiliates
        assert_eq!(render_res.security_tables.get("FOO").unwrap().rows.len(), 2);

        // Filter for default: FOO (1 row) and BAZ visible; BAR omitted
        let render_res = block_on(run_acb_app_to_render_model(
            make_readers(),
            &TxCsvParseOptions::default(),
            Some(AffiliateFilter::new("default")),
            false,
            false,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();
        assert_eq!(render_res.security_tables.len(), 2);
        assert!(render_res.security_tables.contains_key("FOO"));
        assert!(render_res.security_tables.contains_key("BAZ"));
        assert!(!render_res.security_tables.contains_key("BAR"));
        assert_eq!(render_res.security_tables.get("FOO").unwrap().rows.len(), 1);

        // Filter for spouse: FOO (1 row) and BAR visible; BAZ omitted
        let render_res = block_on(run_acb_app_to_render_model(
            make_readers(),
            &TxCsvParseOptions::default(),
            Some(AffiliateFilter::new("spouse")),
            false,
            false,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();
        assert_eq!(render_res.security_tables.len(), 2);
        assert!(render_res.security_tables.contains_key("FOO"));
        assert!(render_res.security_tables.contains_key("BAR"));
        assert!(!render_res.security_tables.contains_key("BAZ"));
        assert_eq!(render_res.security_tables.get("FOO").unwrap().rows.len(), 1);
    }

    #[test]
    fn test_global_stock_split() {
        #[rustfmt::skip]
        let make_readers = || {
            CsvFileBuilder::with_all_modern_headers()
            .split_csv_rows(&vec![6], &vec![
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "1", aps: "1.6", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2016-01-03", sd: "2016-01-05",
                    a: "Buy", sh: "2", aps: "1.6", cur: "CAD", af: "(R)", ..Row::default()},

                // Global split
                Row{sec: "FOO", td: "2016-01-10", sd: "2016-01-10",
                    a: "Split", split: "2-for-1",  af: "", ..Row::default()},

                // Per AF splits (a couple days apart for... the sake of argument)
                Row{sec: "FOO", td: "2017-01-04", sd: "2017-01-04",
                    a: "Split", split: "2-for-1",  af: "Default", ..Row::default()},
                Row{sec: "FOO", td: "2018-01-06", sd: "2018-01-06",
                    a: "Split", split: "2-for-1",  af: "(R)", ..Row::default()},

                Row{sec: "FOO", td: "2018-02-01", sd: "2018-02-03",
                    a: "Buy", sh: "2", aps: "1.6", cur: "CAD", af: "spouse", ..Row::default()},
            ])
        };

        let readers = make_readers();
        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            &TxCsvParseOptions::default(),
            None,
            false,
            false,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();

        let render_table = get_and_check_foo_table(&render_res.security_tables);
        assert_eq!(render_table.rows.len(), 8);
        assert_eq!(Vec::<String>::new(), render_table.errors);
        let row_actions =
            render_table.rows.iter().map(|row| row[3].clone()).collect();
        assert_vec_eq(
            row_actions,
            vec![
                "Buy", "Buy", "Split", "Split", "Split", // duped split
                "Split", "Split", // individual splits
                "Buy",
            ]
            .iter()
            .map(|s| String::from(*s))
            .collect(),
        );

        // Check affiliate filtering
        let affiliate_filter = AffiliateFilter::new("default");
        let readers = make_readers();
        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            &TxCsvParseOptions::default(),
            Some(affiliate_filter),
            false,
            false,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();

        let render_table = get_and_check_foo_table(&render_res.security_tables);
        assert_eq!(render_table.rows.len(), 6);
        assert_eq!(Vec::<String>::new(), render_table.errors);
        let row_actions =
            render_table.rows.iter().map(|row| row[3].clone()).collect();
        assert_vec_eq(
            row_actions,
            vec![
                "Buy", "Buy", "Split", "Split", // duped split
                "Split", "Split", // individual splits
            ]
            .iter()
            .map(|s| String::from(*s))
            .collect(),
        );

        // More affiliate filtering (spouse)
        let affiliate_filter = AffiliateFilter::new("spouse");
        let readers = make_readers();
        let render_res = block_on(run_acb_app_to_render_model(
            readers,
            &TxCsvParseOptions::default(),
            Some(affiliate_filter),
            false,
            false,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();

        let render_table = get_and_check_foo_table(&render_res.security_tables);
        assert_eq!(render_table.rows.len(), 2);
        assert_eq!(Vec::<String>::new(), render_table.errors);
        let row_actions =
            render_table.rows.iter().map(|row| row[3].clone()).collect();
        assert_vec_eq(
            row_actions,
            vec![
                "Split", // duped split
                "Buy",
            ]
            .iter()
            .map(|s| String::from(*s))
            .collect(),
        );
    }
}
