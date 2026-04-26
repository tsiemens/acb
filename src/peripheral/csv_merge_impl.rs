use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use crate::util::basic::SError;

fn normalize_header(header: &str) -> String {
    header.trim().to_lowercase()
}

#[derive(Debug)]
struct OutputColumn {
    header: String,
    normalized_header: String,
}

pub fn merge_csv_files<P, W>(csv_files: &[P], output: W) -> Result<(), SError>
where
    P: AsRef<Path>,
    W: Write,
{
    let mut output_columns = Vec::<OutputColumn>::new();
    let mut seen_output_columns = HashSet::<String>::new();
    let mut output_rows = Vec::<Vec<String>>::new();

    for csv_file in csv_files {
        let csv_path = csv_file.as_ref();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(csv_path)
            .map_err(|e| format!("Failed to read {}: {e}", csv_path.display()))?;

        let headers = reader
            .headers()
            .map_err(|e| {
                format!("Failed to read headers from {}: {e}", csv_path.display())
            })?
            .clone();
        let normalized_headers: Vec<String> =
            headers.iter().map(normalize_header).collect();

        for (header, normalized_header) in headers.iter().zip(&normalized_headers) {
            if seen_output_columns.insert(normalized_header.clone()) {
                output_columns.push(OutputColumn {
                    header: header.to_string(),
                    normalized_header: normalized_header.clone(),
                });
                for row in &mut output_rows {
                    row.push(String::new());
                }
            }
        }

        for (row_index, record) in reader.records().enumerate() {
            let record = record.map_err(|e| {
                format!(
                    "Failed to read row {} from {}: {e}",
                    row_index + 2,
                    csv_path.display()
                )
            })?;
            let mut values = HashMap::<&str, &str>::new();
            for (normalized_header, value) in
                normalized_headers.iter().zip(record.iter())
            {
                values.entry(normalized_header.as_str()).or_insert(value);
            }

            let row = output_columns
                .iter()
                .map(|col| {
                    values
                        .get(col.normalized_header.as_str())
                        .map(|value| (*value).to_string())
                        .unwrap_or_default()
                })
                .collect();
            output_rows.push(row);
        }
    }

    let mut writer = csv::WriterBuilder::new().has_headers(true).from_writer(output);
    writer
        .write_record(output_columns.iter().map(|col| col.header.as_str()))
        .map_err(|e| format!("Failed to write output header: {e}"))?;
    for row in output_rows {
        writer
            .write_record(row)
            .map_err(|e| format!("Failed to write output row: {e}"))?;
    }
    writer.flush().map_err(|e| format!("Failed to flush output: {e}"))?;

    Ok(())
}
