use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;

use time::Date;

#[cfg(target_arch = "wasm32")]
use crate::portfolio::Tx;
#[cfg(not(target_arch = "wasm32"))]
use crate::write_errln;
use crate::{
    app::approot::approot_common::{self, Error, Options},
    fx::io::RateLoader,
    portfolio::{
        io::tx_csv::write_txs_to_csv,
        summary::{make_aggregate_summary_txs, CollectedSummaryData},
        Security, TxDelta,
    },
    util::rw::{DescribedReader, WriteHandle},
};

/// Result type for summary mode, containing structured summary data, CSV text,
/// and warnings/errors.
#[cfg(target_arch = "wasm32")]
pub struct AppSummaryRenderResult {
    pub summary_txs: Vec<Tx>,
    pub csv_text: String,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

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

async fn run_acb_app_summary_to_model(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    options: Options,
    rate_loader: &mut RateLoader,
    err_printer: WriteHandle,
) -> Result<CollectedSummaryData, AppSummaryError> {
    let deltas_results_by_sec = approot_common::run_acb_app_to_delta_models(
        csv_file_readers,
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
        options.affiliate_render_filter,
    ))
}

/// Runs the summary mode and returns a structured result and CSV text output.
///
/// Target user: WASM bindings
#[cfg(target_arch = "wasm32")]
pub async fn run_acb_app_summary_to_render_model(
    latest_date: Date,
    csv_file_readers: Vec<DescribedReader>,
    options: Options,
    rate_loader: &mut RateLoader,
    err_printer: WriteHandle,
) -> AppSummaryRenderResult {
    let res = run_acb_app_summary_to_model(
        latest_date,
        csv_file_readers,
        options,
        rate_loader,
        err_printer.clone(),
    )
    .await;

    match res {
        Ok(summary_data) => {
            // Convert txs to CSV text
            let csv_txs: Vec<crate::portfolio::CsvTx> = summary_data
                .txs
                .iter()
                .map(|tx| crate::portfolio::CsvTx::from(tx.clone()))
                .collect();
            let mut csv_buf = Vec::new();
            let csv_text = match write_txs_to_csv(&csv_txs, &mut csv_buf) {
                Ok(()) => String::from_utf8(csv_buf).unwrap_or_default(),
                Err(e) => format!("Error writing CSV: {e}"),
            };
            AppSummaryRenderResult {
                summary_txs: summary_data.txs,
                csv_text,
                warnings: format_summary_warnings(&summary_data.warnings),
                errors: Vec::new(),
            }
        }
        Err(err_struct) => AppSummaryRenderResult {
            summary_txs: Vec::new(),
            csv_text: String::new(),
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
    rate_loader: &mut RateLoader,
    mut err_printer: WriteHandle,
) -> Result<(), ()> {
    let summ_res = run_acb_app_summary_to_model(
        latest_date,
        csv_file_readers,
        options,
        rate_loader,
        err_printer.clone(),
    )
    .await;

    let summ_data = match summ_res {
        Ok(summ_data) => summ_data,
        Err(err_struct) => {
            let errors = format_summary_errors(&err_struct);
            for error in errors {
                write_errln!(err_printer, "{}", error);
            }
            return Err(());
        }
    };

    let formatted_warnings = format_summary_warnings(&summ_data.warnings);
    if !formatted_warnings.is_empty() {
        write_errln!(err_printer, "Warnings:");
        for warning in formatted_warnings {
            write_errln!(err_printer, " {}", warning);
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
