mod approot_common;
mod calc_approot;
mod summary_approot;

#[cfg(not(target_arch = "wasm32"))]
use crate::{
    app::outfmt::model::AcbWriter,
    fx::io::RateLoader,
    util::rw::{DescribedReader, WriteHandle},
    write_errln,
};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;

// Re-exports for all callers
pub use crate::app::approot::approot_common::Options;
// Re-exports for wasm
#[cfg(target_arch = "wasm32")]
pub use crate::app::approot::summary_approot::run_acb_app_summary_to_render_model;
// Re-exports for tests and wasm
pub use crate::app::approot::calc_approot::run_acb_app_to_writer;

/// This is the main entry point for running the app from the CLI
/// (BOTH regular calculation and summary mode).
#[cfg(not(target_arch = "wasm32"))]
pub async fn run_acb_app_to_console(
    csv_file_readers: Vec<DescribedReader>,
    options: Options,
    rate_loader: &mut RateLoader,
    mut err_printer: WriteHandle,
) -> Result<(), ()> {
    if let Some(summary_mode_latest_date) = options.summary_mode_latest_date {
        summary_approot::run_acb_app_summary_to_console(
            summary_mode_latest_date,
            csv_file_readers,
            options,
            rate_loader,
            err_printer,
        )
        .await
    } else {
        let writer: Box<dyn AcbWriter>;

        if let Some(dir_path) = options.csv_output_dir {
            match super::outfmt::csv::CsvWriter::new_to_output_dir(&dir_path) {
                Ok(w) => writer = Box::new(w),
                Err(e) => {
                    write_errln!(err_printer, "{e}");
                    return Err(());
                }
            }
        } else if let Some(zip_path) = options.csv_output_zip {
            let zip_path = std::path::PathBuf::from(zip_path);
            writer =
                Box::new(super::outfmt::csv::CsvZipWriter::new_to_file(zip_path));
        } else {
            // Default to text writer to stdout
            writer = Box::new(super::outfmt::text::TextWriter::new(
                WriteHandle::stdout_write_handle(),
            ));
        }

        calc_approot::run_acb_app_to_writer(
            writer,
            csv_file_readers,
            &options.csv_parse_options,
            options.affiliate_render_filter,
            options.render_full_dollar_values,
            options.render_total_costs,
            rate_loader,
            err_printer,
        )
        .await
        .map(|_| ())
    }
}
