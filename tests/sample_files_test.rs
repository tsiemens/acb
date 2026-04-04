mod common;

use std::path::Path;

use acb::{
    app::{outfmt::text::TextWriter, run_acb_app_to_writer},
    fx::io::{CsvRatesCache, JsonRemoteRateLoader, RateLoader},
    portfolio::io::tx_csv::TxCsvParseOptions,
    testlib::assert_vec_eq,
    util::{
        date::parse_standard_date,
        http::standalone::StandaloneAppRequester,
        rw::{DescribedReader, WriteHandle},
    },
};
use common::NonAutoCreatingTestDir;

fn validate_sample_csv_file(
    csv_path: &Path,
    cache_dir: &Path,
    render_costs: bool,
    expected_text_path: Option<&Path>,
) {
    let reader = DescribedReader::from_file_path(csv_path.into());

    let (err_stream, err_buff) = WriteHandle::string_buff_write_handle();

    let (write_handle, buff) = WriteHandle::string_buff_write_handle();
    let writer = Box::new(TextWriter::new(write_handle));

    let mut rate_loader = RateLoader::new(
        false,
        Box::new(CsvRatesCache::new(
            cache_dir.to_path_buf(),
            err_stream.clone(),
        )),
        JsonRemoteRateLoader::new_boxed(StandaloneAppRequester::new_boxed()),
        err_stream.clone(),
    );

    let res = async_std::task::block_on(run_acb_app_to_writer(
        writer,
        vec![reader],
        &TxCsvParseOptions::default(),
        false,
        render_costs,
        &mut rate_loader,
        err_stream.clone(),
    ));

    assert_eq!(err_buff.borrow().as_str().to_string(), "");
    res.unwrap();

    if let Some(expected_text_path) = expected_text_path {
        let buff_ref = buff.borrow();
        let text = buff_ref.as_str().to_string();

        let expected_text = std::fs::read_to_string(expected_text_path)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to read expected text file: {:?}",
                    expected_text_path
                )
            });

        assert_vec_eq(
            text.split("\n").collect(),
            expected_text.split("\n").collect(),
        );
    }
}

fn do_test_sample_csv_file_validity(render_costs: bool) {
    acb::util::date::set_todays_date_for_test(
        parse_standard_date("2022-01-01").unwrap(),
    );

    let dir = NonAutoCreatingTestDir::new();

    validate_sample_csv_file(
        Path::new("./tests/data/test_combined.csv"),
        &dir.path,
        render_costs,
        if !render_costs {
            Some(Path::new("./tests/data/test_combined_text.txt"))
        } else {
            None
        },
    );
    validate_sample_csv_file(
        Path::new("./www/static/samples/sample_txs.csv"),
        &dir.path,
        render_costs,
        None,
    );
}

#[test]
fn test_sample_csv_file_validity() {
    do_test_sample_csv_file_validity(false);
    do_test_sample_csv_file_validity(true);
}

#[test]
fn test_sample_csv_parse_errors() {
    acb::util::date::set_todays_date_for_test(
        parse_standard_date("2025-01-01").unwrap(),
    );

    let dir = NonAutoCreatingTestDir::new();
    let csv_path = Path::new("./tests/data/sample_csv_errors.csv");
    let reader = DescribedReader::from_file_path(csv_path.into());

    let (err_stream, err_buff) = WriteHandle::string_buff_write_handle();
    let (write_handle, _buff) = WriteHandle::string_buff_write_handle();
    let writer = Box::new(TextWriter::new(write_handle));

    let mut rate_loader = RateLoader::new(
        false,
        Box::new(CsvRatesCache::new(dir.path.clone(), err_stream.clone())),
        JsonRemoteRateLoader::new_boxed(StandaloneAppRequester::new_boxed()),
        err_stream.clone(),
    );

    let res = async_std::task::block_on(run_acb_app_to_writer(
        writer,
        vec![reader],
        &TxCsvParseOptions::default(),
        false,
        false,
        &mut rate_loader,
        err_stream,
    ));

    assert!(res.is_err(), "Expected error, but got Ok");
    let err_text = err_buff.borrow().as_str().to_string();
    assert!(
        err_text.contains("Invalid action 'InvalidAction'"),
        "err: {}",
        err_text
    );
}
