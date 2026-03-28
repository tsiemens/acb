use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
    str::FromStr,
};

use calamine::{
    open_workbook_auto, open_workbook_auto_from_rs, Data, Range, Reader, Rows,
};
use rust_decimal::{prelude::FromPrimitive, Decimal};

use crate::util::basic::SError;

use super::sheet_common::SheetParseError;

/// Removes all whitespace from a string, used to normalize column names
/// so that extra spaces in xlsx headers (e.g. "Tax  Collection  Shares")
/// still match the expected key ("Tax Collection Shares").
fn strip_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

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
        let key = strip_whitespace(name);
        let col = self.col_name_to_index.get(&key).ok_or_else(|| {
            let mut available: Vec<&str> =
                self.col_name_to_index.keys().map(|s| s.as_str()).collect();
            available.sort();
            self.err(format!(
                "Sheet contained no column '{name}'. \
                 Available columns: {available:?}"
            ))
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

/// Resolves the target sheet name and returns its range from an already-opened
/// workbook. If `sheet_name` is `None` the workbook must contain exactly one
/// sheet; otherwise an error is returned.
///
/// Note: sheet selection cannot be based on index because the calamine API
/// does not guarantee a stable ordering for sheet names.
fn worksheet_from_workbook<RS, R>(
    workbook: &mut R,
    sheet_name: Option<&str>,
) -> Result<Range<Data>, SError>
where
    RS: Read + Seek,
    R: Reader<RS>,
    <R as Reader<RS>>::Error: std::fmt::Display,
{
    let sheet_names: Vec<String>;

    let sheet = if let Some(sn) = sheet_name {
        sn
    } else {
        sheet_names = workbook.sheet_names();
        if sheet_names.len() > 1 {
            return Err(format!(
                "Workbook has more than one sheet: {sheet_names:?}. \
                Sheet name must be specified"
            ));
        }
        sheet_names.get(0).ok_or_else(|| "Workbook has no sheets".to_string())?
    };

    workbook.worksheet_range(sheet).map_err(|e| format!("{e}"))
}

/// Reads the named sheet (or the only sheet) from a workbook file on disk.
///
/// Note: This could not/cannot be based on the sheet index,
/// because the office library does not provide an API to get the
/// sheets in any particular order. They end up coming back in a random
/// order.
pub fn read_xl_file(
    path: &Path,
    sheet_name: Option<&str>,
) -> Result<Range<Data>, SError> {
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("{e}"))?;
    worksheet_from_workbook(&mut workbook, sheet_name)
}

/// Reads the named sheet (or the only sheet) from raw in-memory workbook bytes.
pub fn read_xl_data(
    data: Vec<u8>,
    sheet_name: Option<&str>,
) -> Result<Range<Data>, SError> {
    let cursor = Cursor::new(data);
    let mut workbook =
        open_workbook_auto_from_rs(cursor).map_err(|e| format!("{e}"))?;
    worksheet_from_workbook(&mut workbook, sheet_name)
}

/// Opens a workbook from raw in-memory bytes and reads the specified sheets.
/// Sheets that do not exist in the workbook are silently omitted from the result.
pub fn read_xl_data_sheets(
    data: Vec<u8>,
    sheet_names: &[&str],
) -> Result<HashMap<String, Range<Data>>, SError> {
    let cursor = Cursor::new(data);
    let mut workbook =
        open_workbook_auto_from_rs(cursor).map_err(|e| format!("{e}"))?;
    let available: Vec<String> = workbook.sheet_names();
    let mut result = HashMap::new();
    for &name in sheet_names {
        if available.iter().any(|s| s == name) {
            let range =
                workbook.worksheet_range(name).map_err(|e| format!("{e}"))?;
            result.insert(name.to_string(), range);
        }
    }
    Ok(result)
}

/// Source of an Excel workbook: either a file path or raw in-memory bytes.
pub enum XlSource {
    Path(PathBuf),
    Data(Vec<u8>),
}

/// Opens an Excel workbook from the given source and returns the named sheet
/// (or the only sheet if `sheet_name` is `None`).
pub fn read_xl_source(
    source: XlSource,
    sheet_name: Option<&str>,
) -> Result<Range<Data>, SError> {
    match source {
        XlSource::Path(path) => read_xl_file(&path, sheet_name),
        XlSource::Data(data) => read_xl_data(data, sheet_name),
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
        row_strs.into_iter().enumerate().map(|(i, v)| (strip_whitespace(&v), i)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_whitespace() {
        assert_eq!(
            strip_whitespace("Tax Collection Shares"),
            "TaxCollectionShares"
        );
        assert_eq!(
            strip_whitespace("Tax  Collection  Shares"),
            "TaxCollectionShares"
        );
        assert_eq!(strip_whitespace("NoSpaces"), "NoSpaces");
        assert_eq!(strip_whitespace(" leading"), "leading");
    }

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_sheet_reader_normalizes_whitespace() {
        use rust_xlsxwriter::Workbook;

        let mut wb = Workbook::new();
        let sheet = wb.add_worksheet();
        // Header with extra spaces
        sheet.write(0, 0, "Name").unwrap();
        sheet.write(0, 1, "Tax  Collection  Shares").unwrap();
        sheet.write(0, 2, "Amount").unwrap();
        // Data row
        sheet.write(1, 0, "FOO").unwrap();
        sheet.write(1, 1, 21.0).unwrap();
        sheet.write(1, 2, 100.5).unwrap();

        let data = wb.save_to_buffer().unwrap();
        let range = read_xl_data(data, None).unwrap();
        let mut rows = range.rows();
        let mut reader = SheetReader::new(&mut rows).unwrap();

        let data_row = rows.next().unwrap();
        reader.set_row(data_row, 2);

        // Normal key matches header with extra spaces
        assert_eq!(reader.get_str("Tax Collection Shares").unwrap(), "21");
        // Key with no spaces also works
        assert_eq!(reader.get_str("TaxCollectionShares").unwrap(), "21");
        // Regular columns still work
        assert_eq!(reader.get_str("Name").unwrap(), "FOO");
    }
}
