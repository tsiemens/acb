use std::{collections::HashMap, fs, path::PathBuf};

use acb::{
    fx::{io::{
        pub_testlib::MockRemoteRateLoader,
        CsvRatesCache, RateLoader,
    }, DailyRate},
    tracing,
    util::{date::pub_testlib::doy_date, rc::RcRefCellT, rw::WriteHandle}
};
use rust_decimal_macros::dec;

fn test_temp_dir_path() -> PathBuf {
    let tmpdir = std::env::temp_dir();

    let make_file_path = |val| {
        let fname = format!("acb-test-{}", val);
        tmpdir.join(fname)
    };

    for val in 1..1000000 {
        let path = make_file_path(val);
        if !path.exists() {
            return path;
        }
    }
    panic!("Could not create temp directory path that does not already exist");
}

struct NonAutoCreatingTestDir {
    pub path: PathBuf
}

impl NonAutoCreatingTestDir {
    pub fn new() -> NonAutoCreatingTestDir {
        NonAutoCreatingTestDir{path: test_temp_dir_path()}
    }
}

fn cleanup_test_dir(path: &PathBuf) {
    if path.exists() {
        let skip_env_var = "SKIP_TEMP_DIR_CLEANUP_ON_FAIL";
        let skip_del_on_fail = acb::util::sys::env_var_non_empty(skip_env_var);

        if std::thread::panicking() && skip_del_on_fail {
            println!("cleanup_test_dir: panicking. Skipping remove of {}",
                     path.to_str().unwrap());
        } else {
            println!("cleanup_test_dir: removing {}. To skip cleanup, set {}",
                     path.to_str().unwrap(), skip_env_var);
            let _ = fs::remove_dir_all(path);
        }
    } else {
        println!("cleanup_test_dir: {} did not exist", path.to_str().unwrap());
    }
}

impl Drop for NonAutoCreatingTestDir {
    fn drop(&mut self) {
        cleanup_test_dir(&self.path);
    }
}

#[test]
fn test_get_effective_usd_cad_rate_with_csv_cache() {
    tracing::setup_tracing();

    let dir = NonAutoCreatingTestDir::new();

    let remote_year_rates =  RcRefCellT::new(HashMap::new());
    let mut rate_loader = RateLoader::new(
        true, // force download
        Box::new(CsvRatesCache::new(dir.path.clone(), WriteHandle::empty_write_handle())),
        Box::new(MockRemoteRateLoader{ remote_year_rates: remote_year_rates.clone() }),
        WriteHandle::empty_write_handle());

    // rate_loader.remote_loader
    remote_year_rates.borrow_mut().insert(
        2022, vec![DailyRate::new(doy_date(2022, 1), dec!(1.2))]);

	// fetch mocked remote values and write to cache
    let rate = rate_loader.blocking_get_effective_usd_cad_rate(doy_date(2022, 1)).unwrap();
    assert_eq!(rate, DailyRate::new(doy_date(2022, 1), dec!(1.2)));

	// remove remote values to ensure cache is used
    remote_year_rates.borrow_mut().insert(2022, vec![]);

    let mut cached_rate_loader = RateLoader::new(
        false,
        Box::new(CsvRatesCache::new(dir.path.clone(), WriteHandle::empty_write_handle())),
        Box::new(MockRemoteRateLoader{ remote_year_rates: remote_year_rates.clone() }),
        WriteHandle::empty_write_handle());

    let rate = cached_rate_loader.blocking_get_effective_usd_cad_rate(doy_date(2022, 1)).unwrap();
    assert_eq!(rate, DailyRate::new(doy_date(2022, 1), dec!(1.2)));
}