use std::borrow::BorrowMut;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::str::FromStr;

use rust_decimal::Decimal;

use crate::util::date::parse_standard_date;
use crate::util::decimal::LessEqualZeroDecimal;
use crate::util::rw::DescribedReader;
use crate::{log::WriteHandle, write_errln};
use crate::portfolio::csv_common::CsvCol;
use crate::portfolio::{Affiliate, CsvTx, Currency, SFLInput, TxAction};

type Error = String;

fn parse_csv_action(value: &str) -> Result<TxAction, Error> {
    match value.trim().to_lowercase().as_str() {
        "buy" => Ok(TxAction::Buy),
        "sell" => Ok(TxAction::Sell),
        "roc" => Ok(TxAction::Roc),
        "sfla" => Ok(TxAction::Sfla),
        _ => Err(format!("Invalid action '{value}'")),
    }
}

fn parse_csv_superficial_loss(value: &str) -> Result<SFLInput, Error> {
	// Check for forcing marker (a terminating !)
    let force_flag = value.ends_with("!");
    let num_view = if force_flag {
        &value[..value.len() - 1]
    } else {
        value
    };

    let number = Decimal::from_str(num_view).map_err(
        |_| format!("Invalid number in {}: {}",
        CsvCol::SUPERFICIAL_LOSS, num_view))?;
    let constrained_number = LessEqualZeroDecimal::try_from(number)
        .map_err(|_| format!("Invalid {} {}: Was positive value",
                                        CsvCol::SUPERFICIAL_LOSS, number))?;
    Ok(SFLInput{
        superficial_loss: constrained_number,
        force: force_flag,
    })
}

fn csvtx_from_csv_values(mut values: HashMap::<&str, String>, read_index: u32) -> Result<CsvTx, Error> {
    let parse_decimal = |value: &str, field_name: &str| {
        Decimal::from_str(value).map_err(
            |e| format!("Failed to parse number for {} ('{}'): {}",
                                         field_name, value, e))
    };

    Ok(CsvTx{
        security: values.remove(CsvCol::SECURITY),
        trade_date: match values.remove(CsvCol::TRADE_DATE) {
            Some(s) => Some(parse_standard_date(&s).map_err(
                |e| format!("Failed to parse {} \"{}\": {}",
                    CsvCol::TRADE_DATE, s, e))?),
            None => None,
        },
        settlement_date: {
            let s_date = match values.remove(CsvCol::SETTLEMENT_DATE) {
                Some(s) => Some(parse_standard_date(&s).map_err(
                    |e| format!("Failed to parse {} \"{}\": {}",
                        CsvCol::SETTLEMENT_DATE, s, e))?),
                None => None,
            };
            let legacy_s_date = match values.remove(CsvCol::LEGACY_SETTLEMENT_DATE) {
                Some(s) => Some(parse_standard_date(&s).map_err(
                    |e| format!("Failed to parse {} \"{}\": {}",
                        CsvCol::SETTLEMENT_DATE, s, e))?),
                None => None,
            };
            if s_date.is_some() { s_date } else { legacy_s_date }
        },
        action: match values.remove(CsvCol::ACTION) {
            Some(s) => Some(parse_csv_action(&s)?),
            None => None,
        },
        shares: match values.remove(CsvCol::SHARES) {
            Some(s) => Some(parse_decimal(&s, CsvCol::SHARES)?),
            None => None,
        },
        amount_per_share: match values.remove(CsvCol::AMOUNT_PER_SHARE) {
            Some(s) => Some(parse_decimal(&s, CsvCol::AMOUNT_PER_SHARE)?),
            None => None,
        },
        commission: match values.remove(CsvCol::COMMISSION) {
            Some(s) => Some(parse_decimal(&s, CsvCol::COMMISSION)?),
            None => None,
        },
        tx_currency: match values.remove(CsvCol::TX_CURR) {
            Some(s) => Some(Currency::new(&s)),
            None => None,
        },
        tx_curr_to_local_exchange_rate: match values.remove(CsvCol::TX_FX) {
            Some(s) => Some(parse_decimal(&s, CsvCol::TX_FX)?),
            None => None,
        },
        commission_currency: match values.remove(CsvCol::COMMISSION_CURR) {
            Some(s) => Some(Currency::new(&s)),
            None => None,
        },
        commission_curr_to_local_exchange_rate: match values.remove(CsvCol::COMMISSION_FX) {
            Some(s) => Some(parse_decimal(&s, CsvCol::COMMISSION_FX)?),
            None => None,
        },
        memo: match values.remove(CsvCol::MEMO) {
            Some(s) => Some(s),
            None => None,
        },
        affiliate: match values.remove(CsvCol::AFFILIATE) {
            Some(s) => Some(Affiliate::from_strep(&s)),
            None => None,
        },
        specified_superficial_loss: match values.remove(CsvCol::SUPERFICIAL_LOSS) {
            Some(s) => Some(parse_csv_superficial_loss(&s)?),
            None => None,
        },
        read_index: read_index,
    })
}

pub fn parse_tx_csv(
    desc_reader: &mut DescribedReader,
    initial_global_read_index: u32,
    err_stream: &mut WriteHandle,
    ) -> Result<Vec<CsvTx>, Error> {

    let mut reader_box = desc_reader.reader().map_err(
        |e| e.to_string())?;
    let reader: &mut dyn Read = reader_box.borrow_mut();

    let mut csv_r = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);

    let csv_desc = desc_reader.desc();

    let mut col_index_to_name: HashMap::<usize, &'static str> = HashMap::new();
    let mut found_col_names: HashSet::<&'static str> = HashSet::new();

    let col_names = CsvCol::get_csv_cols();

    let headers_res = csv_r.headers().map_err(
        |e| format!("Error in csv headers of {csv_desc}: {e}"))?;
    for (i, col) in headers_res.iter().enumerate() {
        let lower_col = col.to_lowercase();
        let san_col = lower_col.trim();
        match col_names.get(san_col) {
            Some(static_str) => {
                col_index_to_name.insert(i, static_str);
                found_col_names.insert(static_str);
            },
            None => {
                write_errln!(err_stream, "Warning: Unrecognized column in {csv_desc}: {san_col}");
            },
        }
    }

    // Finalize these
    let col_index_to_name = col_index_to_name;

    // Sanity check columns
    if found_col_names.contains(CsvCol::SETTLEMENT_DATE) &&
       found_col_names.contains(CsvCol::LEGACY_SETTLEMENT_DATE) {
        return Err(format!("{} contains both '{}' and '{}' (deprecated) columns",
            csv_desc, CsvCol::SETTLEMENT_DATE, CsvCol::LEGACY_SETTLEMENT_DATE));
    }

    let mut txs = Vec::<CsvTx>::new();

    let mut global_row_index = initial_global_read_index;

    for (i, record_res) in csv_r.records().enumerate() {
        // Start at 1 for the user, and include header.
        let row_num = i + 2;

        let record = match record_res {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("Error reading rates csv record in {csv_desc} at row {row_num}: {e}"));
            },
        };

        let mut tx_values = HashMap::<&'static str, String>::new();
        for (i, col_val) in record.iter().enumerate() {
            if !col_val.trim().is_empty() {
                match col_index_to_name.get(&i) {
                    Some(col_name) => { tx_values.insert(col_name, col_val.trim().to_string()); },
                    None => (), // This entire column was not recognized, so ignore it.
                }
            }
        }

        let tx = csvtx_from_csv_values(tx_values, global_row_index).map_err(
            |e| format!("Error on row {row_num} of {csv_desc}: {e}"))?;
        txs.push(tx);

        global_row_index += 1;
    }

    Ok(txs)

}

#[cfg(test)]
pub mod testlib {
    use crate::{portfolio::csv_common::CsvCol, util::rw::DescribedReader};

    // The names here are abbreviated to make test writing as concise and
    // convenient as possible.
    #[derive(Default)]
    pub struct TestTxCsvRow {
        pub sec: &'static str, // SECURITY
        pub td: &'static str, // TRADE_DATE
        pub sd: &'static str, // SETTLEMENT_DATE
        pub legacy_date: &'static str, // LEGACY_SETTLEMENT_DATE
        pub a: &'static str, // ACTION
        pub sh: &'static str, // SHARES
        pub aps: &'static str, // AMOUNT_PER_SHARE
        pub com: &'static str, // COMMISSION
        pub cur: &'static str, // TX_CURR
        pub fx: &'static str, // TX_FX
        pub c_cur: &'static str, // COMMISSION_CURR
        pub c_fx: &'static str, // COMMISSION_FX
        pub sfl: &'static str, // SUPERFICIAL_LOSS
        pub af: &'static str, // AFFILIATE
        pub m: &'static str, // MEMO
    }

    impl TestTxCsvRow {
        pub fn get_col(&self, col: &str) -> &'static str {
            match col {
                CsvCol::SECURITY => self.sec,
                CsvCol::TRADE_DATE => self.td,
                CsvCol::SETTLEMENT_DATE => self.sd,
                CsvCol::LEGACY_SETTLEMENT_DATE => self.legacy_date,
                CsvCol::ACTION => self.a,
                CsvCol::SHARES => self.sh,
                CsvCol::AMOUNT_PER_SHARE => self.aps,
                CsvCol::COMMISSION => self.com,
                CsvCol::TX_CURR => self.cur,
                CsvCol::TX_FX => self.fx,
                CsvCol::COMMISSION_CURR => self.c_cur,
                CsvCol::COMMISSION_FX => self.c_fx,
                CsvCol::SUPERFICIAL_LOSS => self.sfl,
                CsvCol::AFFILIATE => self.af,
                CsvCol::MEMO => self.m,
                _ => panic!("Invalid col {}", col),
            }
        }

        pub fn make_row_line(&self, cols: &Vec<&'static str>) -> String {
            let mut parts = Vec::new();
            for col in cols {
                let part = self.get_col(*col);
                if part.contains(",") {
                    panic!("test tx col {} value '{}' contained a comma, which is not supported",
                           col, part);
                }
                parts.push(part);
            }
            parts.join(",")
        }
    }

    pub enum CsvFileBuilder {
        WithHeaders(Vec<&'static str>),
        WithStringHeaders(Vec<String>),
    }

    impl CsvFileBuilder {
        pub fn with_custom_header_line(h: &str) -> CsvFileBuilder {
            CsvFileBuilder::WithStringHeaders(h.split(",").map(|s| s.to_string()).collect())
        }

        //         const LEGACY_HEADER: &'static str = "security,date,action,shares,amount/share,currency,exchange rate,commission,memo,superficial loss";
        //         const STD_HEADER: &'static str = "security,trade date,settlement date,action,shares,amount/share,\
        // currency,exchange rate,commission,affiliate,memo,superficial loss";

        pub fn with_headers(hs: Vec<&'static str>) -> CsvFileBuilder {
            CsvFileBuilder::WithHeaders(hs)
        }

        pub fn with_all_modern_headers() -> Self {
            let mut headers = CsvCol::get_csv_cols();
            headers.remove(CsvCol::LEGACY_SETTLEMENT_DATE);
            CsvFileBuilder::WithHeaders(Vec::from_iter(headers.iter().map(|f| *f)))
        }


        fn csv_reader_from_rows<T: ToString>(&self, desc: String, rows: std::slice::Iter<T>) -> DescribedReader {
            let strs: Vec<String> = rows.map(|s| s.to_string()).collect();
            let contents = strs.join("\n");
            let headers = self.header_str_and_newline();
            let full_content = headers.to_owned() + &contents;
            DescribedReader::from_string(desc, full_content)
        }

        fn header_str(&self) -> String {
            match self {
                CsvFileBuilder::WithHeaders(hs) => {
                    hs.join(",")
                },
                CsvFileBuilder::WithStringHeaders(hs) => {
                    hs.join(",")
                }
            }
        }

        fn header_str_and_newline(&self) -> String {
            self.header_str() + "\n"
        }

        pub fn headers(&self) -> Vec<String> {
            self.header_str().split(",").map(|s| s.to_string()).collect()
        }

        pub fn static_headers(&self) -> &Vec<&'static str> {
            match self {
                CsvFileBuilder::WithHeaders(hs) => hs,
                // This function is only used with TestTxCsvRow, and we always have
                // valid, static headers in this case. Invalid header test cases should
                // use raw row strings.
                _ => panic!("Cannot use static_headers here"),
            }
        }

        pub fn make_row_strings(&self, tx_rows: &Vec<TestTxCsvRow>) -> Vec<String> {
            let header_list = self.static_headers();
            let mut rows: Vec<String> = Vec::new();
            for tx_row in tx_rows {
                rows.push(tx_row.make_row_line(header_list));
            }
            rows
        }

        pub fn single_csv_reader_raw<T: ToString>(&self, rows: &Vec<T>)
            -> DescribedReader {
            self.split_raw_csv_rows(&vec![rows.len()], rows).remove(0)
        }

        pub fn split_raw_csv_rows<T: ToString>(&self, file_lens: &Vec<usize>, rows: &Vec<T>)
            -> Vec<DescribedReader> {
            let mut rows_read: usize = 0;
            let mut csv_readers = Vec::new();
            for (i, file_len) in file_lens.iter().enumerate() {
                csv_readers.push(self.csv_reader_from_rows(
                    format!("foo{i}.csv"),
                    rows[rows_read..rows_read+(*file_len)].iter()
                ));
                rows_read += *file_len;
            }
            csv_readers
        }

        pub fn single_csv_reader(&self, rows: &Vec<TestTxCsvRow>)
            -> DescribedReader {
            self.split_csv_rows(&vec![rows.len()], rows).remove(0)
        }

        pub fn split_csv_rows(&self, file_lens: &Vec<usize>, tx_rows: &Vec<TestTxCsvRow>)
            -> Vec<DescribedReader> {
            let rows = self.make_row_strings(tx_rows);
            self.split_raw_csv_rows(file_lens, &rows)
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::{
        log::WriteHandle,
        portfolio::{io::tx_csv::testlib::TestTxCsvRow, Affiliate, CsvTx, Currency, SFLInput},
        testlib::assert_vecr_eq,
        util::{date::parse_standard_date, decimal::LessEqualZeroDecimal}
    };

    use super::{parse_csv_superficial_loss, parse_tx_csv, testlib::CsvFileBuilder, Error};

    type Row = TestTxCsvRow;

    #[test]
    fn test_parse_sfl() {
        let sfl_ = |n, f| {
            SFLInput{
                superficial_loss: LessEqualZeroDecimal::try_from(n).unwrap(),
                force: f,
            }
        };

        assert_eq!(
            parse_csv_superficial_loss("0!").unwrap(),
            sfl_(dec!(0), true));
        assert_eq!(
            parse_csv_superficial_loss("-1.1!").unwrap(),
            sfl_(dec!(-1.1), true));
        assert_eq!(
            parse_csv_superficial_loss("0.0").unwrap(),
            sfl_(dec!(0), false));
        assert_eq!(
            parse_csv_superficial_loss("-1.1").unwrap(),
            sfl_(dec!(-1.1), false));

        assert_eq!(
            parse_csv_superficial_loss("").unwrap_err(),
            "Invalid number in superficial loss: ");
        assert_eq!(
            parse_csv_superficial_loss("sfsd").unwrap_err(),
            "Invalid number in superficial loss: sfsd");
        assert_eq!(
            parse_csv_superficial_loss("1.1").unwrap_err(),
            "Invalid superficial loss 1.1: Was positive value");
        assert_eq!(
            parse_csv_superficial_loss("1.1!").unwrap_err(),
            "Invalid superficial loss 1.1: Was positive value");
        assert_eq!(
            parse_csv_superficial_loss("!-1.1").unwrap_err(),
            "Invalid number in superficial loss: !-1.1");
    }

    #[test]
    fn test_double_settlement_date() {
        let mut d_reader =
            CsvFileBuilder::with_custom_header_line(
                "security,date,settlement date\n")
            .single_csv_reader_raw(&vec![
                "FOO,2016-01-03,2016-01-05",
                "BAR,2016-01-03,2016-01-06",
            ]);
        let err = parse_tx_csv(
            &mut d_reader, 0,
            &mut WriteHandle::empty_write_handle()).unwrap_err();
        assert_eq!(err, "foo0.csv contains both 'settlement date' and 'date' (deprecated) columns");
    }

    #[test]
    fn test_double_unknown_columns() {
        let mut d_reader =
            CsvFileBuilder::with_custom_header_line(
                "blabla,security\n")
            .single_csv_reader_raw(&vec![
                ",FOO",
                "lksjlksjdf,BAR",
            ]);

        let (mut err_writer, buff) = WriteHandle::string_buff_write_handle();
        let txs = parse_tx_csv(&mut d_reader, 10, &mut err_writer).unwrap();

        assert_vecr_eq(&txs, &vec![
            CsvTx{security: Some("FOO".to_string()), read_index: 10, ..CsvTx::default()},
            CsvTx{security: Some("BAR".to_string()), read_index: 11, ..CsvTx::default()},
        ]);
        assert_eq!(
            buff.borrow().as_str(),
            "Warning: Unrecognized column in foo0.csv: blabla\n"
        );
    }

    #[test]
    fn test_parse_tx_csv_basic() {
        let mut d_reader = CsvFileBuilder::with_all_modern_headers()
            .single_csv_reader(&vec![
                Row{sec:"Foo ",td:"2020-11-11",sd:"2020-11-13",legacy_date:"",a:"Buy",sh:"123.1",aps:"10.1",
                    com: "20.1", cur: "USD", fx: "1.3",c_cur: "usd", c_fx: "1.31",
                    sfl: "-1.2!", af: "(R)", m:"A memo",
                },
                Row::default(),
                // Empty sec after trimming
                Row{sec:"   ", ..Default::default()},
                // Other action casing
                Row{a:"SFLA", ..Default::default()},
                Row{a:"sfla", ..Default::default()},
            ]);
        let txs = parse_tx_csv(
            &mut d_reader, 0,
            &mut WriteHandle::empty_write_handle()).unwrap();

        let exp_txs = vec![
            CsvTx{
                security: Some("Foo".to_string()),
                trade_date: Some(parse_standard_date("2020-11-11").unwrap()),
                settlement_date: Some(parse_standard_date("2020-11-13").unwrap()),
                action: Some(crate::portfolio::TxAction::Buy),
                shares: Some(dec!(123.1)),
                amount_per_share: Some(dec!(10.1)),
                commission: Some(dec!(20.1)),
                tx_currency: Some(Currency::usd()),
                tx_curr_to_local_exchange_rate: Some(dec!(1.3)),
                commission_currency: Some(Currency::usd()),
                commission_curr_to_local_exchange_rate: Some(dec!(1.31)),
                memo: Some("A memo".to_string()),
                affiliate: Some(Affiliate::default_registered()),
                specified_superficial_loss: Some(SFLInput::req_from_dec(dec!(-1.2), true)),
                read_index: 0,
            },
            CsvTx{read_index: 1, ..CsvTx::default()},
            CsvTx{read_index: 2, ..CsvTx::default()},
            CsvTx{action: Some(crate::portfolio::TxAction::Sfla), read_index: 3, ..CsvTx::default()},
            CsvTx{action: Some(crate::portfolio::TxAction::Sfla), read_index: 4, ..CsvTx::default()},
        ];

        assert_vecr_eq(&txs, &exp_txs);
    }

    #[test]
    fn test_parse_tx_csv_fatal_errors() {
        let parse_fatal_row = |row| -> Error {
            let mut d_reader = CsvFileBuilder::with_all_modern_headers()
                .single_csv_reader(&vec![row]);
            parse_tx_csv(
                &mut d_reader, 0,
                &mut WriteHandle::empty_write_handle()).unwrap_err()
        };

        // Illegal date
        let err = parse_fatal_row(Row{td:"2020-20-20", ..Default::default()});
        assert_eq!(err, "Error on row 2 of foo0.csv: Failed to parse trade date \"2020-20-20\": the 'month' component could not be parsed");
        let err = parse_fatal_row(Row{sd:"2020-20-20", ..Default::default()});
        assert_eq!(err, "Error on row 2 of foo0.csv: Failed to parse settlement date \"2020-20-20\": the 'month' component could not be parsed");
        // Invalid action
        let err = parse_fatal_row(Row{a:"bla", ..Default::default()});
        assert_eq!(err, "Error on row 2 of foo0.csv: Invalid action 'bla'");
        // Invalid decimal value
        let err = parse_fatal_row(Row{sh:"bla", ..Default::default()});
        assert_eq!(err, "Error on row 2 of foo0.csv: Failed to parse number for shares ('bla'): Invalid decimal: unknown character");
        // Invalid SFL
        let err = parse_fatal_row(Row{sfl:"1", ..Default::default()});
        assert_eq!(err, "Error on row 2 of foo0.csv: Invalid superficial loss 1: Was positive value");
    }

    #[test]
    fn test_to_tx_csv_string() {
        let mut d_reader =
        CsvFileBuilder::with_custom_header_line(
            "security,trade date,settlement date,action,shares,amount/share,commission,currency,\
			exchange rate,commission currency,commission exchange rate,superficial loss,affiliate,memo\n"
        )
            .single_csv_reader_raw(&vec![
                "FOO,2016-01-03,2016-01-05,Sell,5,1.6,0,CAD,,CAD,,,Default,a memo",
			    "BAR,2016-01-03,2016-01-06,Buy,7,1.7,1,USD,1.11,USD,1.11,,Default,a memo 2",
			    "AA,2016-01-04,2016-01-07,Sell,1,1.7,1,USD,1.11,USD,1.11,-1.2,Default,M3",
			    "BB,2016-01-05,2016-01-08,Sell,2,1.7,1,USD,1.11,USD,1.11,-1.3!,Default (R),M4",
			    "CC,2016-01-08,2016-01-10,SfLA,2,1.3,0,CAD,,CAD,,,B,M5",
            ]);
        let _ = parse_tx_csv(
            &mut d_reader, 0,
            &mut WriteHandle::empty_write_handle()).unwrap();

        // TODO
    }
}