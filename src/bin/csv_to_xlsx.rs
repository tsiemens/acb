use std::{collections::HashMap, path::PathBuf};

use acb::util::{basic::SError, date::parse_standard_date};
use clap::Parser;
use rust_xlsxwriter::{Format, Workbook};

fn add_csv_file_as_sheet(wb: &mut Workbook, csv_fname: &str) -> Result<(), SError> {
    let csv_path = PathBuf::from(csv_fname);

    let title = csv_path
        .file_name()
        .map(|os_name| os_name.to_str())
        .unwrap_or(None)
        .map(|name| name.split(".").next().unwrap());

    let mut csv_r = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(csv_fname)
        .map_err(|e| e.to_string())?;

    let sheet = wb.add_worksheet();
    if let Some(t) = title {
        let _ = sheet.set_name(t);
    }

    let date_format = Format::new().set_num_format("yyyy-mm-dd");

    let mut col_widths = HashMap::<u16, f64>::new();

    for (r_i, row_res) in csv_r.records().enumerate() {
        let row = row_res.map_err(|e| format!("Row {r_i}: {e}"))?;
        for (c_i, cell_str) in row.iter().enumerate() {
            let row_i: u32 = r_i.try_into().unwrap();
            let col_i: u16 = c_i.try_into().unwrap();

            let old_width = col_widths.get(&col_i).map(|v| *v).unwrap_or(0.0);
            col_widths.insert(col_i, old_width.max(cell_str.len() as f64));

            if let Ok(date) = parse_standard_date(cell_str) {
                let date_data = rust_xlsxwriter::ExcelDateTime::from_ymd(
                    date.year().try_into().unwrap(),
                    date.month().into(),
                    date.day(),
                )
                .unwrap();
                sheet
                    .write_with_format(row_i, col_i, &date_data, &date_format)
                    .map_err(|e| e.to_string())?;
            } else if let Ok(num) = cell_str.parse::<f64>() {
                sheet.write(row_i, col_i, num).map_err(|e| e.to_string())?;
            } else {
                sheet.write(row_i, col_i, cell_str).map_err(|e| e.to_string())?;
            }
        }
    }

    for (col, width) in col_widths {
        let _ = sheet.set_column_width(col, width);
    }

    Ok(())
}

/// A convenience script to convert csv to xlsx
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// One or more CSV files. Each will be placed in its own sheet
    #[arg(required = true)]
    pub csv_files: Vec<String>,

    #[arg(long, default_value_t = String::from("out.xlsx"))]
    pub output_filename: String,
}

fn main() -> Result<(), SError> {
    let mut args = Args::parse();

    // Args itself should ensure this happens gracefully.
    assert!(args.csv_files.len() > 0);
    args.csv_files.sort();

    let mut workbook = Workbook::new();
    for f_name in args.csv_files.iter() {
        add_csv_file_as_sheet(&mut workbook, f_name.as_str())?;
    }
    workbook.save(args.output_filename).map_err(|e| e.to_string())?;
    Ok(())
}
