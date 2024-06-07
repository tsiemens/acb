mod common;

use std::collections::HashMap;

use acb::{
    fx::{
        io::{pub_testlib::MockRemoteRateLoader, CsvRatesCache, RateLoader},
        DailyRate,
    },
    tracing,
    util::{date::pub_testlib::doy_date, rc::RcRefCellT, rw::WriteHandle},
};
use common::NonAutoCreatingTestDir;
use rust_decimal_macros::dec;

#[test]
fn test_get_effective_usd_cad_rate_with_csv_cache() {
    tracing::setup_tracing();

    let dir = NonAutoCreatingTestDir::new();

    let remote_year_rates = RcRefCellT::new(HashMap::new());
    let mut rate_loader = RateLoader::new(
        true, // force download
        Box::new(CsvRatesCache::new(
            dir.path.clone(),
            WriteHandle::empty_write_handle(),
        )),
        Box::new(MockRemoteRateLoader {
            remote_year_rates: remote_year_rates.clone(),
        }),
        WriteHandle::empty_write_handle(),
    );

    // rate_loader.remote_loader
    remote_year_rates
        .borrow_mut()
        .insert(2022, vec![DailyRate::new(doy_date(2022, 1), dec!(1.2))]);

    // fetch mocked remote values and write to cache
    let rate =
        rate_loader.blocking_get_effective_usd_cad_rate(doy_date(2022, 1)).unwrap();
    assert_eq!(rate, DailyRate::new(doy_date(2022, 1), dec!(1.2)));

    // remove remote values to ensure cache is used
    remote_year_rates.borrow_mut().insert(2022, vec![]);

    let mut cached_rate_loader = RateLoader::new(
        false,
        Box::new(CsvRatesCache::new(
            dir.path.clone(),
            WriteHandle::empty_write_handle(),
        )),
        Box::new(MockRemoteRateLoader {
            remote_year_rates: remote_year_rates.clone(),
        }),
        WriteHandle::empty_write_handle(),
    );

    let rate = cached_rate_loader
        .blocking_get_effective_usd_cad_rate(doy_date(2022, 1))
        .unwrap();
    assert_eq!(rate, DailyRate::new(doy_date(2022, 1), dec!(1.2)));
}
