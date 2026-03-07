use std::{collections::HashMap, str::FromStr};

use calamine::{Data, Rows};
use rust_decimal::{prelude::FromPrimitive, Decimal};

use super::sheet_common::SheetParseError;

pub struct SheetReader<'a> {
    col_name_to_index: HashMap<String, usize>,
    row: Option<&'a [Data]>,
    // This should be 1-index based
    row_num: usize,
}

impl<'a> SheetReader<'a> {
    pub fn new(rows: &mut Rows<'_, Data>) -> Result<Self, SheetParseError> {
        let col_name_to_index = read_sheet_header(rows)?;

        Ok(SheetReader {
            col_name_to_index,
            row: None,
            row_num: 0,
        })
    }

    pub fn set_row(&mut self, r: &'a [Data], row_num: usize) {
        if row_num == 0 {
            panic!("row_num was 0");
        }
        self.row = Some(r);
        self.row_num = row_num;
    }

    pub fn get(&self, name: &str) -> Result<&Data, SheetParseError> {
        let col = self.col_name_to_index.get(name).ok_or_else(|| {
            self.err(format!("Sheet contained no column '{name}'"))
        })?;
        let v: &Data = self.row.unwrap().get(*col).unwrap();
        Ok(v)
    }

    pub fn get_str(&self, name: &str) -> Result<String, SheetParseError> {
        Ok(match self.get(name)? {
            Data::String(s) => s.clone(),
            Data::Bool(b) => b.to_string(),
            Data::Error(e) => format!("{e:?}"),
            Data::Empty => String::new(),
            Data::Int(v) => v.to_string(),
            Data::Float(v) => v.to_string(),
            Data::DateTime(dt) => dt.to_string(),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
        })
    }

    pub fn get_opt_dec(
        &self,
        name: &str,
    ) -> Result<Option<Decimal>, SheetParseError> {
        Ok(match self.get(name)? {
            Data::Int(v) => Some(Decimal::from_i64(*v).ok_or(
                self.err(format!("{v} in {name} unconvertible to Decimal")),
            )?),
            Data::Float(v) => Some(Decimal::from_f64(*v).ok_or(
                self.err(format!("{v} in {name} unconvertible to Decimal")),
            )?),
            Data::String(s) => Some(Decimal::from_str(s).map_err(|e| {
                self.err(format!(
                    "Unable to parse number from \"{s}\" in {name}: {e}"
                ))
            })?),
            Data::Bool(b) => {
                return Err(
                    self.err(format!("{b} in {name} not convertible to Decimal"))
                );
            }
            Data::Error(e) => {
                return Err(self.err(format!("Error in {name}: {e:?}")));
            }
            Data::Empty => None,
            Data::DateTime(dt) => Some(Decimal::from_f64(dt.as_f64()).ok_or(
                self.err(format!("{dt} in {name} unconvertible to Decimal")),
            )?),
            Data::DateTimeIso(s) | Data::DurationIso(s) => {
                Some(Decimal::from_str(s).map_err(|e| {
                    self.err(format!(
                        "Unable to parse number from \"{s}\" in {name}: {e}"
                    ))
                })?)
            }
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
fn read_sheet_header(
    rows: &mut Rows<'_, Data>,
) -> Result<HashMap<String, usize>, SheetParseError> {
    let first_row = match rows.next() {
        Some(r) => r,
        None => return Err(SheetParseError::new(1, format!("Sheet was empty"))),
    };

    let row_strs: Vec<String> = first_row
        .into_iter()
        .filter(|cell| match &cell {
            Data::String(_) => true,
            _ => false,
        })
        .map(|cell| match cell {
            Data::String(s) => s.clone(),
            v => panic!("Data was {v:?}"),
        })
        .collect();

    Ok(HashMap::from_iter(
        row_strs.into_iter().enumerate().map(|(i, v)| (v, i)),
    ))
}
