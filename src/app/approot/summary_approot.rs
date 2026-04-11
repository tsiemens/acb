use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;

use time::Date;

use crate::{
    app::{
        approot::approot_common::{self, AppRenderMode, Error, Options},
        config::AcbConfig,
    },
    fx::io::RateLoader,
    portfolio::{
        io::tx_csv::write_txs_to_csv,
        summary::{make_aggregate_summary_txs, CollectedSummaryData},
        Affiliate, AffiliateFilter, CsvTx, Security, Tx, TxDelta,
    },
    util::rw::{DescribedReader, WriteHandle},
};

#[cfg(not(target_arch = "wasm32"))]
use crate::write_errln;

pub struct AppSummaryRenderVariant {
    pub summary_txs: Vec<Tx>,
    pub csv_text: String,
}

pub struct AppSummaryRenderByAffiliateVariants {
    pub unfiltered: AppSummaryRenderVariant,
    pub by_affiliate: HashMap<String, AppSummaryRenderVariant>,
}

pub enum AppSummaryRenderOutput {
    Default(AppSummaryRenderVariant),
    ByAffiliate(AppSummaryRenderByAffiliateVariants),
}

/// Result type for summary mode, containing structured summary data, CSV text,
/// and warnings/errors.
#[cfg(target_arch = "wasm32")]
pub struct AppSummaryRenderResult {
    pub output: AppSummaryRenderOutput,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct AppSummaryError {
    pub general_error: Option<Error>,
    pub sec_errors: HashMap<Security, Error>,
}

/// Converts warnings from HashMap<String, Vec<String>> (in CollectedSummaryData)
/// to Vec<String> for display.
fn format_summary_warnings(warnings: &HashMap<String, Vec<String>>) -> Vec<String> {
    warnings
        .iter()
        .map(|(warning, secs)| {
            if secs.is_empty() {
                format!("{}.", warning)
            } else {
                format!("{}. Encountered for {}", warning, secs.join(", "))
            }
        })
        .collect()
}

fn format_summary_errors(errors_struct: &AppSummaryError) -> Vec<String> {
    let mut errors = Vec::new();
    if let Some(e) = &errors_struct.general_error {
        errors.push(format!("Error: {}", e));
    }
    for (sec, e) in &errors_struct.sec_errors {
        errors.push(format!("Error in {}: {}", sec, e));
    }
    errors
}

fn txs_to_csv_text(txs: &[Tx]) -> String {
    let csv_txs: Vec<CsvTx> = txs.iter().map(|tx| CsvTx::from(tx.clone())).collect();
    let mut csv_buf = Vec::new();
    match write_txs_to_csv(&csv_txs, &mut csv_buf) {
        Ok(()) => String::from_utf8(csv_buf).unwrap_or_default(),
        Err(e) => format!("Error writing CSV: {e}"),
    }
}

fn make_summary_variant(txs: Vec<Tx>) -> AppSummaryRenderVariant {
    let csv_text = txs_to_csv_text(&txs);
    AppSummaryRenderVariant {
        summary_txs: txs,
        csv_text,
    }
}

fn apply_affiliate_filter(txs: Vec<Tx>, filter: &AffiliateFilter) -> Vec<Tx> {
    txs.into_iter().filter(|tx| filter.matches(&tx.affiliate)).collect()
}

/// Runs the summary app mode.
/// Returns the rendered output and any warnings
async fn run_acb_app_summary_to_model(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    options: Options,
    config: Option<&AcbConfig>,
    app_render_mode: AppRenderMode,
    rate_loader: &mut RateLoader,
    err_printer: WriteHandle,
) -> Result<(AppSummaryRenderOutput, Vec<String>), AppSummaryError> {
    let deltas_results_by_sec = approot_common::run_acb_app_to_delta_models(
        csv_file_readers,
        &options.csv_parse_options,
        config,
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

    let summary_data: CollectedSummaryData = make_aggregate_summary_txs(
        latest_date,
        &deltas_by_sec,
        options.split_annual_summary_gains,
    );

    let formatted_warnings = format_summary_warnings(&summary_data.warnings);

    let output = match app_render_mode {
        AppRenderMode::Default => {
            let txs = match options.affiliate_render_filter {
                None => summary_data.txs,
                Some(filter) => apply_affiliate_filter(summary_data.txs, &filter),
            };
            AppSummaryRenderOutput::Default(make_summary_variant(txs))
        }
        AppRenderMode::ByAffiliateIfMultiple => {
            let mut af_base_names: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for tx in &summary_data.txs {
                if !tx.affiliate.is_global() {
                    // We must ue the normalized base name, since it's possible that
                    // the natural base name for the registered and unregistered
                    // variants could have different capitalization.
                    af_base_names.insert(tx.affiliate.base_name_normalized());
                }
            }

            if af_base_names.len() <= 1 {
                AppSummaryRenderOutput::Default(make_summary_variant(
                    summary_data.txs,
                ))
            } else {
                let unfiltered_variant =
                    make_summary_variant(summary_data.txs.clone());

                let mut by_affiliate = HashMap::new();
                for base_name in af_base_names {
                    if let Some(rfilter) = &options.affiliate_render_filter {
                        if !rfilter
                            .matches(&Affiliate::from_base_name(&base_name, false))
                        {
                            continue;
                        }
                    }

                    let filter = AffiliateFilter::new(&base_name);
                    let filtered_txs =
                        apply_affiliate_filter(summary_data.txs.clone(), &filter);
                    by_affiliate
                        .insert(base_name, make_summary_variant(filtered_txs));
                }

                AppSummaryRenderOutput::ByAffiliate(
                    AppSummaryRenderByAffiliateVariants {
                        unfiltered: unfiltered_variant,
                        by_affiliate,
                    },
                )
            }
        }
    };

    Ok((output, formatted_warnings))
}

/// Runs the summary mode and returns a structured result and CSV text output.
///
/// Target user: WASM bindings
#[cfg(target_arch = "wasm32")]
pub async fn run_acb_app_summary_to_render_model(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    options: Options,
    config: Option<&AcbConfig>,
    rate_loader: &mut RateLoader,
    err_printer: WriteHandle,
) -> AppSummaryRenderResult {
    let res = run_acb_app_summary_to_model(
        latest_date,
        csv_file_readers,
        options,
        config,
        AppRenderMode::Default,
        rate_loader,
        err_printer.clone(),
    )
    .await;

    match res {
        Ok((output, warnings)) => AppSummaryRenderResult {
            output,
            warnings,
            errors: Vec::new(),
        },
        Err(err_struct) => AppSummaryRenderResult {
            output: AppSummaryRenderOutput::Default(AppSummaryRenderVariant {
                summary_txs: Vec::new(),
                csv_text: String::new(),
            }),
            warnings: Vec::new(),
            errors: format_summary_errors(&err_struct),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_acb_app_summary_to_console(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    options: Options,
    config: Option<&AcbConfig>,
    rate_loader: &mut RateLoader,
    mut err_printer: WriteHandle,
) -> Result<(), ()> {
    let res = run_acb_app_summary_to_model(
        latest_date,
        csv_file_readers,
        options,
        config,
        AppRenderMode::Default,
        rate_loader,
        err_printer.clone(),
    )
    .await;

    let (output, warnings) = match res {
        Ok(result) => result,
        Err(err_struct) => {
            let errors = format_summary_errors(&err_struct);
            for error in errors {
                write_errln!(err_printer, "{}", error);
            }
            return Err(());
        }
    };

    if !warnings.is_empty() {
        write_errln!(err_printer, "Warnings:");
        for warning in &warnings {
            write_errln!(err_printer, " {}", warning);
        }
        write_errln!(err_printer, "");
    }

    let csv_text = match output {
        AppSummaryRenderOutput::Default(variant) => variant.csv_text,
        AppSummaryRenderOutput::ByAffiliate(variants) => {
            variants.unfiltered.csv_text
        }
    };

    if !csv_text.is_empty() {
        if let Err(e) =
            WriteHandle::stdout_write_handle().write_all(csv_text.as_bytes())
        {
            write_errln!(err_printer, "Error writing output: {e}");
            return Err(());
        }
    }

    Ok(())
}

// MARK: Tests
#[cfg(test)]
mod tests {
    use async_std::task::block_on;
    use time::Month;

    use crate::portfolio::io::tx_csv::testlib::TestTxCsvRow as Row;
    use crate::portfolio::AffiliateFilter;
    use crate::{
        app::approot::approot_common::{AppRenderMode, Options},
        fx::io::{InMemoryRatesCache, JsonRemoteRateLoader, RateLoader},
        portfolio::io::tx_csv::testlib::CsvFileBuilder,
        util::{http::testlib::UnusableHttpRequester, rw::WriteHandle},
    };

    use super::{run_acb_app_summary_to_model, AppSummaryRenderOutput};

    fn make_empty_test_rate_loader() -> RateLoader {
        RateLoader::new(
            false,
            Box::new(InMemoryRatesCache::new()),
            JsonRemoteRateLoader::new_boxed(UnusableHttpRequester::new_boxed()),
            WriteHandle::empty_write_handle(),
        )
    }

    fn make_date(year: i32, month: u8, day: u8) -> time::Date {
        time::Date::from_calendar_date(year, Month::try_from(month).unwrap(), day)
            .unwrap()
    }

    fn get_default_summary_txs_len(output: &AppSummaryRenderOutput) -> usize {
        match output {
            AppSummaryRenderOutput::Default(v) => v.summary_txs.len(),
            _ => panic!("Expected Default render output"),
        }
    }

    #[test]
    fn test_multi_affiliate_filtering() {
        // FOO: transactions in default and spouse affiliates
        // BAR: transactions only in spouse affiliate
        #[rustfmt::skip]
        let make_readers = || {
            CsvFileBuilder::with_all_modern_headers()
            .split_csv_rows(&vec![3], &vec![
                Row{sec: "FOO", td: "2020-01-10", sd: "2020-01-14",
                    a: "Buy", sh: "10", aps: "1.0", cur: "CAD", ..Row::default()},
                Row{sec: "FOO", td: "2020-01-11", sd: "2020-01-15",
                    a: "Buy", sh: "5", aps: "2.0", cur: "CAD", af: "spouse", ..Row::default()},
                Row{sec: "BAR", td: "2020-01-10", sd: "2020-01-14",
                    a: "Buy", sh: "3", aps: "1.0", cur: "CAD", af: "spouse", ..Row::default()},
            ])
        };

        let latest_date = make_date(2024, 12, 31);

        // No filter: summary txs for all affiliates in both securities
        // (BAR:spouse, FOO:default, FOO:spouse in sorted security order)
        let (output, _warnings) = block_on(run_acb_app_summary_to_model(
            latest_date,
            make_readers(),
            Options::default(),
            None,
            AppRenderMode::Default,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();
        assert_eq!(get_default_summary_txs_len(&output), 3);

        // Filter for "default": only FOO's default tx survives
        let (output, _warnings) = block_on(run_acb_app_summary_to_model(
            latest_date,
            make_readers(),
            Options {
                affiliate_render_filter: Some(AffiliateFilter::new("Default")),
                ..Options::default()
            },
            None,
            AppRenderMode::Default,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();
        let txs = match &output {
            AppSummaryRenderOutput::Default(v) => &v.summary_txs,
            _ => panic!("Expected Default render output"),
        };
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].affiliate.name(), "Default");

        // Filter for "spouse": FOO's spouse tx and BAR's spouse tx survive
        let (output, _warnings) = block_on(run_acb_app_summary_to_model(
            latest_date,
            make_readers(),
            Options {
                affiliate_render_filter: Some(AffiliateFilter::new("spouse")),
                ..Options::default()
            },
            None,
            AppRenderMode::Default,
            &mut make_empty_test_rate_loader(),
            WriteHandle::empty_write_handle(),
        ))
        .unwrap();
        let txs = match &output {
            AppSummaryRenderOutput::Default(v) => &v.summary_txs,
            _ => panic!("Expected Default render output"),
        };
        assert_eq!(txs.len(), 2);
        for tx in txs {
            assert_eq!(tx.affiliate.name(), "spouse");
        }
    }
}
