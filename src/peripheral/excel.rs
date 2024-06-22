use std::{collections::HashMap, str::FromStr};

use office::DataType;
use rust_decimal::{prelude::FromPrimitive, Decimal};

use super::sheet_common::SheetParseError;

pub struct SheetReader<'a> {
    // rows: office::Rows<'a>,

    col_name_to_index: HashMap<String, usize>,
    row: Option<&'a[DataType]>,
    // This should be 1-index based
    row_num: usize,
}

impl <'a> SheetReader<'a> {
    pub fn new(rows: &mut office::Rows) -> Result<Self, SheetParseError> {
        let col_name_to_index = read_sheet_header(rows)?;

        Ok(SheetReader { col_name_to_index, row: None, row_num: 0 })
    }

    pub fn set_row(&mut self, r: &'a[DataType], row_num: usize) {
        if row_num == 0 {
            panic!("row_num was 0");
        }
        self.row = Some(r);
        self.row_num = row_num;
    }

    pub fn get(&self, name: &str) -> Result<&DataType, SheetParseError> {
        let col = self.col_name_to_index.get(name).ok_or_else(
            || self.err(format!("Sheet contained no column '{name}'")))?;
        let v: &DataType = self.row.unwrap().get(*col).unwrap();
        Ok(v)
    }

    pub fn get_str(&self, name: &str) -> Result<String, SheetParseError> {
        Ok(match self.get(name)? {
            DataType::String(s) => s.clone(),
            DataType::Bool(b) => b.to_string(),
            DataType::Error(e) => format!("{e:?}"),
            DataType::Empty => String::new(),
            DataType::Int(v) => v.to_string(),
            DataType::Float(v) => v.to_string(),
        })
    }

    pub fn get_opt_dec(&self, name: &str)
    -> Result<Option<Decimal>, SheetParseError> {
        Ok(match self.get(name)? {
            DataType::Int(v) =>
                Some(Decimal::from_i64(*v).ok_or(
                    self.err(format!("{v} in {name} unconvertible to Decimal")))?),
            DataType::Float(v) =>
                Some(Decimal::from_f64(*v).ok_or(
                    self.err(format!("{v} in {name} unconvertible to Decimal")))?),
            DataType::String(s) =>
                Some(Decimal::from_str(s).map_err(|e| self.err(
                    format!("Unable to parse number from \"{s}\" in {name}: {e}")))?),
            DataType::Bool(b) => {
                return Err(self.err(
                    format!("{b} in {name} not convertible to Decimal")));
            },
            DataType::Error(e) => {
                return Err(self.err(format!("Error in {name}: {e:?}")));
            },
            DataType::Empty => None,
        })
    }

    pub fn get_dec(&self, name: &str) -> Result<Decimal, SheetParseError> {
        match self.get_opt_dec(name) {
            Ok(o) => o.ok_or(self.err(format!("value in {name} was empty"))),
            Err(e) => Err(e),
        }
    }

    fn err(&self, s: String) -> SheetParseError {
        SheetParseError::new(self.row_num, s)
    }
}

/// Reads the first row of the range, and returns a mapping of
/// column name to index
fn read_sheet_header(rows: &mut office::Rows)
-> Result<HashMap<String, usize>, SheetParseError> {
    let first_row = match rows.next() {
        Some(r) => r,
        None => return Err(
            SheetParseError::new(1, format!("Sheet was empty"))),
    };

    let row_strs: Vec<String> = first_row.into_iter()
        .filter(|cell| match &cell {
            DataType::String(_) => true,
            _ => false,
        })
        .map(|cell| match cell {
            DataType::String(s) => s.clone(),
            v => panic!("DataType was {v:?}"),
        })
        .collect();

    Ok(HashMap::from_iter(row_strs.into_iter().enumerate()
        .map(|(i, v)| (v, i))))
}