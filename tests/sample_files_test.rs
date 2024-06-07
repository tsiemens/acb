mod common;

use std::{collections::HashMap, path::Path};

use acb::{
    app::run_acb_app_to_render_model,
    fx::io::{CsvRatesCache, JsonRemoteRateLoader, RateLoader},
    portfolio::io::tx_csv::TxCsvParseOptions,
    util::{
        date::parse_standard_date,
        http::standalone::StandaloneAppRequester,
        rw::{DescribedReader, WriteHandle},
    },
};
use common::NonAutoCreatingTestDir;

fn validate_sample_csv_file(csv_path: &Path, cache_dir: &Path, render_costs: bool) {
    let reader = DescribedReader::from_file_path(csv_path.into());

    let err_stream = WriteHandle::empty_write_handle();

    async_std::task::block_on(run_acb_app_to_render_model(
        vec![reader],
        HashMap::new(),
        &TxCsvParseOptions::default(),
        false,
        render_costs,
        RateLoader::new(
            false,
            Box::new(CsvRatesCache::new(
                cache_dir.to_path_buf(),
                err_stream.clone(),
            )),
            // Box::new(MockRemoteRateLoader{
            //     remote_year_rates: RcRefCellT::new(HashMap::new()) }),
            JsonRemoteRateLoader::new_boxed(StandaloneAppRequester::new_boxed()),
            err_stream.clone(),
        ),
        err_stream.clone(),
    ))
    .unwrap();
}

fn do_test_sample_csv_file_validity(render_costs: bool) {
    acb::util::date::set_todays_date_for_test(
        parse_standard_date("2022-01-01").unwrap(),
    );

    let dir = NonAutoCreatingTestDir::new();

    validate_sample_csv_file(
        Path::new("./pytest/test_combined.csv"),
        &dir.path,
        render_costs,
    );
    validate_sample_csv_file(
        Path::new("./www/html/sample_txs.csv"),
        &dir.path,
        render_costs,
    );
}

#[test]
fn test_sample_csv_file_validity() {
    do_test_sample_csv_file_validity(false);
    do_test_sample_csv_file_validity(true);
}
