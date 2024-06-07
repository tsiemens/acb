use std::collections::HashMap;

use crate::{
    fx::DailyRate,
    util::{
        basic::SError,
        rc::{RcRefCell, RcRefCellT},
    },
};

pub trait RatesCache {
    fn write_rates(
        &mut self,
        year: u32,
        rates: &Vec<DailyRate>,
    ) -> Result<(), SError>;
    fn get_usd_cad_rates(
        &mut self,
        year: u32,
    ) -> Result<Option<Vec<DailyRate>>, SError>;
}

pub struct InMemoryRatesCache {
    pub rates_by_year: RcRefCell<HashMap<u32, Vec<DailyRate>>>,
}

impl InMemoryRatesCache {
    pub fn new() -> InMemoryRatesCache {
        InMemoryRatesCache {
            rates_by_year: RcRefCellT::new(HashMap::new()),
        }
    }
}

impl RatesCache for InMemoryRatesCache {
    fn write_rates(
        &mut self,
        year: u32,
        rates: &Vec<DailyRate>,
    ) -> Result<(), SError> {
        (*self.rates_by_year.borrow_mut()).insert(year, rates.clone());
        Ok(())
    }

    fn get_usd_cad_rates(
        &mut self,
        year: u32,
    ) -> Result<Option<Vec<DailyRate>>, SError> {
        Ok(self.rates_by_year.borrow().get(&year).map(|f| f.clone()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod csv {
    use std::{
        fs::File,
        io::{Read, Write},
        path::PathBuf,
        str::FromStr,
    };

    use rust_decimal::Decimal;
    use tracing::{error, info, trace};

    use crate::util::basic::SError;
    use crate::{
        fx::DailyRate,
        util::{date, rw::WriteHandle},
        write_errln,
    };

    use super::RatesCache;

    pub struct CsvRatesCache {
        // Typically, this will be wherever get_home_dir() provides,
        // but can be specified for integration testing.
        dir_path: std::path::PathBuf,
        err_writer: WriteHandle,
    }

    impl CsvRatesCache {
        pub fn new(
            dir_path: std::path::PathBuf,
            err_writer: WriteHandle,
        ) -> CsvRatesCache {
            CsvRatesCache {
                dir_path: dir_path,
                err_writer: err_writer,
            }
        }

        fn get_rates_from_csv(
            &mut self,
            r: &mut dyn Read,
        ) -> Result<Vec<DailyRate>, SError> {
            let mut csv_r =
                csv::ReaderBuilder::new().has_headers(false).from_reader(r);

            let mut rates: Vec<DailyRate> = Vec::new();

            for record_res in csv_r.records() {
                let record = match record_res {
                    Ok(r) => r,
                    Err(e) => {
                        write_errln!(
                            self.err_writer,
                            "Error reading rates csv record: {}",
                            e
                        );
                        continue;
                    }
                };
                let date_val = match record.get(0) {
                    Some(ds) => match date::parse_standard_date(ds) {
                        Ok(d) => d,
                        Err(e) => {
                            write_errln!(
                                self.err_writer,
                                "Error parsing rates csv date: {}",
                                e
                            );
                            continue;
                        }
                    },
                    None => {
                        write_errln!(
                            self.err_writer,
                            "Error reading rates from csv: Row has no fields"
                        );
                        continue;
                    }
                };

                let rate = match record.get(1) {
                    Some(ds) => match Decimal::from_str(ds) {
                        Ok(d) => d,
                        Err(e) => {
                            write_errln!(
                                self.err_writer,
                                "Error parsing rates csv rate for {}: {}",
                                date_val,
                                e
                            );
                            continue;
                        }
                    },
                    None => {
                        write_errln!(
                            self.err_writer,
                            "Error reading rates from csv: {} has no rate",
                            date_val
                        );
                        continue;
                    }
                };

                rates.push(DailyRate {
                    date: date_val,
                    foreign_to_local_rate: rate,
                })
            }

            Ok(rates)
        }
    }

    fn rates_csv_file_path(dir_path: &std::path::Path, year: u32) -> PathBuf {
        let fname_only = format!("rates-{}.csv", year);
        dir_path.join(fname_only)
    }

    fn open_rates_csv_file_write(
        dir_path: &std::path::Path,
        year: u32,
    ) -> Result<File, SError> {
        let file_path = rates_csv_file_path(dir_path, year);
        crate::util::os::mk_writable_dir(dir_path).map_err(|e| e.to_string())?;
        File::create(file_path).map_err(|e| e.to_string())
    }

    fn open_rates_csv_file_read(
        dir_path: &std::path::Path,
        year: u32,
    ) -> Result<Option<File>, SError> {
        let file_path = rates_csv_file_path(dir_path, year);
        match File::open(file_path) {
            Ok(f) => Ok(Some(f)),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(e.to_string()),
            },
        }
    }

    impl RatesCache for CsvRatesCache {
        fn write_rates(
            &mut self,
            year: u32,
            rates: &Vec<DailyRate>,
        ) -> Result<(), SError> {
            info!(
                "CsvRatesCache::write_rates {} into {}",
                year,
                if let Some(p) = self.dir_path.to_str() {
                    p
                } else {
                    "<no path ???>"
                }
            );
            let file = open_rates_csv_file_write(&self.dir_path, year)?;

            // CSV file of date,exchange_rate

            let mut csv_w = csv::Writer::from_writer(file);
            for rate in rates {
                csv_w
                    .write_record(vec![
                        rate.date.to_string(),
                        rate.foreign_to_local_rate.to_string(),
                    ])
                    .map_err(|e| e.to_string())?;
            }
            let r = csv_w.flush().map_err(|e| e.to_string());
            if r.is_ok() {
                trace!("CsvRatesCache::write_rates flushed ok");
            } else {
                error!(
                    "CsvRatesCache::write_rates failed flushed: {}",
                    r.as_ref().err().unwrap()
                );
            }
            r
        }

        fn get_usd_cad_rates(
            &mut self,
            year: u32,
        ) -> Result<Option<Vec<DailyRate>>, SError> {
            trace!(year = year, "CsvRatesCache::get_usd_cad_rates");
            let mut file_opt = open_rates_csv_file_read(&self.dir_path, year)?;
            match &mut file_opt {
                Some(file) => self.get_rates_from_csv(file).map(|v| Some(v)),
                None => Ok(None),
            }
        }
    }

    #[cfg(test)]
    mod csv_tests {
        use std::path::PathBuf;

        use rust_decimal_macros::dec;

        use crate::{
            fx::DailyRate,
            testlib::{assert_re, assert_vec_eq},
            util::{date::pub_testlib::doy_date, rw::WriteHandle},
        };

        use super::CsvRatesCache;

        #[test]
        fn test_read_csv() {
            let date_yd = |year, doy| doy_date(year, doy);
            let dr = |date, rate| DailyRate {
                date: date,
                foreign_to_local_rate: rate,
            };

            let (write_handle, err_buff) = WriteHandle::string_buff_write_handle();

            let loader_path = PathBuf::from("/tmp/acb-non-existant");
            let mut loader = CsvRatesCache::new(loader_path.clone(), write_handle);

            let b = String::from(
                "2022-01-01,1.12
2022-01-02,1.13
2022-01-03,1.14",
            );
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_vec_eq(
                res.unwrap(),
                vec![
                    dr(date_yd(2022, 0), dec!(1.12)),
                    dr(date_yd(2022, 1), dec!(1.13)),
                    dr(date_yd(2022, 2), dec!(1.14)),
                ],
            );
            assert_eq!(err_buff.borrow().as_str(), "");

            // Empty csv
            let b = String::new();
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_vec_eq(res.unwrap(), vec![]);
            assert_eq!(err_buff.borrow().as_str(), "");

            // Empty rows (they are ignored)
            let b = String::from(
                "
2022-01-02,1.12

2022-01-02,1.13
",
            );
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_vec_eq(
                res.unwrap(),
                vec![
                    dr(date_yd(2022, 1), dec!(1.12)),
                    dr(date_yd(2022, 1), dec!(1.13)),
                ],
            );
            assert_eq!(err_buff.borrow().as_str(), "");

            // Empty date
            let b = String::from(
                ",
2022-01-02,1.13
",
            );
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_vec_eq(res.unwrap(), vec![dr(date_yd(2022, 1), dec!(1.13))]);
            assert_re("Error parsing rates csv date:", err_buff.borrow().as_str());
            err_buff.borrow_mut().clear();

            // Inconsistent columns
            let b = String::from(
                "2022-01-02
2022-01-02,1.13
",
            );
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_re(
                "has no rate.*\n.*previous record has 1 field",
                err_buff.borrow().as_str(),
            );
            assert_vec_eq(res.unwrap(), vec![]);
            err_buff.borrow_mut().clear();

            // No value column
            let b = String::from("2022-01-02");
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_re("has no rate", err_buff.borrow().as_str());
            assert_vec_eq(res.unwrap(), vec![]);
            err_buff.borrow_mut().clear();

            // Bad rate value
            let b = String::from(
                "2022-01-02,lksjdf
2022-01-02,1.13
",
            );
            let res = loader.get_rates_from_csv(&mut b.as_bytes());

            assert_re("Invalid decimal", err_buff.borrow().as_str());
            assert_vec_eq(res.unwrap(), vec![dr(date_yd(2022, 1), dec!(1.13))]);
            err_buff.borrow_mut().clear();

            // Cleanup. Make sure that this unit test didn't create the loader directory,
            // since it is not doing writes or reads.
            assert!(!loader_path.exists());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use csv::*;
