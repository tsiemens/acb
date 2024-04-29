use std::{collections::HashMap, fs::File, io::{Read, Write}, path::PathBuf, str::FromStr};

use rust_decimal::Decimal;

use crate::{
    fx::DailyRate,
    log::WriteHandle,
    util::{date, sys::home_dir_file_path}
};

use super::Error;

pub trait RatesCache {
    fn write_rates(&mut self, year: u32, rates: &Vec<DailyRate>) -> Result<(), Error>;
    fn get_usd_cad_rates(&mut self, year: u32) -> Result<Option<Vec<DailyRate>>, Error>;
}

pub struct InMemoryRatesCache {
    pub rates_by_year: HashMap<u32, Vec<DailyRate>>,
}

impl InMemoryRatesCache {
    pub fn new() -> InMemoryRatesCache {
        InMemoryRatesCache{rates_by_year: HashMap::new()}
    }
}

impl RatesCache for InMemoryRatesCache {
    fn write_rates(&mut self, year: u32, rates: &Vec<DailyRate>) -> Result<(), Error> {
        self.rates_by_year.insert(year, rates.clone());
        Ok(())
    }

    fn get_usd_cad_rates(&mut self, year: u32) -> Result<Option<Vec<DailyRate>>, Error> {
        Ok(self.rates_by_year.get(&year).map(|f| f.clone()))
    }
}

pub struct CsvRatesCache {
    err_writer: WriteHandle,
}

impl CsvRatesCache {
    pub fn new(err_writer: WriteHandle) -> CsvRatesCache {
        CsvRatesCache{err_writer: err_writer}
    }

    fn get_rates_from_csv(&mut self, r: &mut dyn Read) -> Result<Vec<DailyRate>, Error> {
        let mut csv_r = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(r);

        let mut rates: Vec<DailyRate> = Vec::new();

        for record_res in csv_r.records() {
            let record = match record_res {
                Ok(r) => r,
                Err(e) => {
                    let _ = writeln!(self.err_writer, "Error reading rates csv record: {}", e);
                    continue;
                },
            };
            let date_val = match record.get(0) {
                Some(ds) => {
                    match date::parse_standard_date(ds) {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = writeln!(self.err_writer, "Error parsing rates csv date: {}", e);
                            continue;
                        },
                    }
                },
                None => {
                    let _ = writeln!(self.err_writer, "Error reading rates from csv: Row has no fields");
                    continue;
                },
            };

            let rate = match record.get(1) {
                Some(ds) => {
                    match Decimal::from_str(ds) {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = writeln!(self.err_writer,
                                "Error parsing rates csv rate for {}: {}", date_val, e);
                            continue;
                        },
                    }
                },
                None => {
                    let _ = writeln!(self.err_writer,
                        "Error reading rates from csv: {} has no rate", date_val);
                    continue;
                },
            };

            rates.push(DailyRate{date: date_val, foreign_to_local_rate: rate})
        }

        Ok(rates)
    }
}

fn open_rates_csv_file(year: u32, write_mode: bool) -> Result<File, Error> {
    let fname_only = format!("rates-{}.csv", year);
    let fname_only_path = PathBuf::from(fname_only);
    let file_path = home_dir_file_path(&fname_only_path)?;

    (if write_mode {
        File::create(file_path)
    } else {
        File::open(file_path)
    }).map_err(|e| e.to_string())
}

impl RatesCache for CsvRatesCache {
    fn write_rates(&mut self, year: u32, rates: &Vec<DailyRate>) -> Result<(), Error> {
        let file = open_rates_csv_file(year, true)?;

        // CSV file of date,exchange_rate

        let mut csv_w = csv::Writer::from_writer(file);
        for rate in rates {
            csv_w.write_record(vec![rate.date.to_string(), rate.foreign_to_local_rate.to_string()])
                .map_err(|e|e.to_string())?;
        }
        csv_w.flush()
            .map_err(|e|e.to_string())
    }

    fn get_usd_cad_rates(&mut self, year: u32) -> Result<Option<Vec<DailyRate>>, Error> {
        let mut file = open_rates_csv_file(year, false)?;
        // TODO what if the file doesn't exist yet?
        self.get_rates_from_csv(&mut file).map(|v| Some(v))
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::{
        fx::DailyRate,
        log::WriteHandle,
        testlib::{assert_re, assert_vec_eq},
        util::date::testlib::doy_date
    };

    use super::CsvRatesCache;

    #[test]
    fn test_read_csv() {
        let date_yd = |year, doy| { doy_date(year, doy) };
        let dr = |date, rate| { DailyRate{date: date, foreign_to_local_rate: rate} };

        let (write_handle, err_buff) = WriteHandle::string_buff_write_handle();

        let mut loader = CsvRatesCache::new(write_handle);

        let b = String::from("2022-01-01,1.12
2022-01-02,1.13
2022-01-03,1.14");
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_vec_eq(
            res.unwrap(),
            vec![
                dr(date_yd(2022, 0), dec!(1.12)),
                dr(date_yd(2022, 1), dec!(1.13)),
                dr(date_yd(2022, 2), dec!(1.14)),
            ]
        );
        assert_eq!(err_buff.borrow().as_str(), "");

        // Empty csv
        let b = String::new();
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_vec_eq(
            res.unwrap(),
            vec![]
        );
        assert_eq!(err_buff.borrow().as_str(), "");

        // Empty rows (they are ignored)
        let b = String::from("
2022-01-02,1.12

2022-01-02,1.13
");
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_vec_eq(
            res.unwrap(),
            vec![
                dr(date_yd(2022, 1), dec!(1.12)),
                dr(date_yd(2022, 1), dec!(1.13)),
            ]
        );
        assert_eq!(err_buff.borrow().as_str(), "");

        // Empty date
        let b = String::from(",
2022-01-02,1.13
");
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_vec_eq(
            res.unwrap(),
            vec![
                dr(date_yd(2022, 1), dec!(1.13)),
            ]
        );
        assert_re("Error parsing rates csv date:", err_buff.borrow().as_str());
        err_buff.borrow_mut().clear();

        // Inconsistent columns
        let b = String::from("2022-01-02
2022-01-02,1.13
");
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_re("has no rate.*\n.*previous record has 1 field", err_buff.borrow().as_str());
        assert_vec_eq(
            res.unwrap(),
            vec![]
        );
        err_buff.borrow_mut().clear();

        // No value column
        let b = String::from("2022-01-02");
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_re("has no rate", err_buff.borrow().as_str());
        assert_vec_eq(
            res.unwrap(),
            vec![]
        );
        err_buff.borrow_mut().clear();

        // Bad rate value
        let b = String::from("2022-01-02,lksjdf
2022-01-02,1.13
");
        let res = loader.get_rates_from_csv(&mut b.as_bytes());

        assert_re("Invalid decimal", err_buff.borrow().as_str());
        assert_vec_eq(
            res.unwrap(),
            vec![
                dr(date_yd(2022, 1), dec!(1.13)),
            ]
        );
        err_buff.borrow_mut().clear();
    }
}