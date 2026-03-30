use std::collections::HashSet;
use std::path::Path;

use lazy_static::lazy_static;

use crate::portfolio::csv_common::CsvCol;
use crate::util::basic::SError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    // Not really for peripheral input, though it is an output.
    AcbTxCsv,
    // Any Questrade xls, xlsx (Excel parsable) activities file.
    QuestradeExcel,
    // RBC Direct Investing CSV activities export.
    RbcDiCsv,
    // Etrade
    EtradeTradeConfirmationPdf,
    EtradeBenefitPdf,
    EtradeBenefitsExcel,
    // Other
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDetectResult {
    pub kind: FileKind,
    /// An optional warning hint, typically set when the kind is Unknown
    /// to explain why detection failed (e.g. which columns were missing).
    pub warning: Option<String>,
}

impl FileDetectResult {
    fn ok(kind: FileKind) -> Self {
        FileDetectResult {
            kind,
            warning: None,
        }
    }

    fn unknown(warning: String) -> Self {
        FileDetectResult {
            kind: FileKind::Unknown,
            warning: Some(warning),
        }
    }

    fn unknown_bare() -> Self {
        FileDetectResult {
            kind: FileKind::Unknown,
            warning: None,
        }
    }
}

/// Input source for file detection.
pub enum FileDetectSource<'a> {
    /// A file path on disk.
    Path(&'a Path),
    /// Raw file bytes and the original file name (for extension detection).
    Bytes { data: &'a [u8], file_name: &'a str },
    /// Pre-parsed PDF pages (each element is one page's text).
    PdfPages(&'a [String]),
}

lazy_static! {
    static ref ETRADE_BENEFIT_PDF_PATTERN: regex::Regex = regex::Regex::new(
        r"STOCK\s+PLAN\s+(RELEASE|EXERCISE)\s+CONFIRMATION|Plan\s*(2014|ESP2)"
    )
    .unwrap();
    static ref ETRADE_TRADE_CONF_PDF_PATTERN: regex::Regex = regex::Regex::new(
        r"TRADE\s*CONFIRMATION|This\s+transaction\s+is\s+confirmed"
    )
    .unwrap();
}

#[cfg(feature = "xlsx_read")]
/// Questrade header columns we check for to identify a Questrade activities export.
const QUESTRADE_REQUIRED_HEADERS: &[&str] =
    &["Transaction Date", "Action", "Symbol", "Quantity", "Account #"];

/// Detect the kind of broker/ACB file from the given source.
///
/// For `Path` and `Bytes` sources, this reads the file content as needed.
/// For `PdfPages`, only PDF-based detection is performed.
///
/// The returned `FileDetectResult` includes an optional warning hint
/// when detection fails, explaining why (e.g. which headers were missing).
pub fn detect_file_kind(
    source: FileDetectSource,
) -> Result<FileDetectResult, SError> {
    match source {
        FileDetectSource::PdfPages(pages) => Ok(detect_from_pdf_text(pages)),
        FileDetectSource::Path(path) => detect_from_path(path),
        FileDetectSource::Bytes { data, file_name } => {
            detect_from_bytes(data, file_name)
        }
    }
}

fn file_extension(name: &str) -> &str {
    Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("")
}

fn detect_from_path(path: &Path) -> Result<FileDetectResult, SError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "csv" => {
            let data = std::fs::read(path)
                .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
            Ok(detect_csv(&data))
        }
        #[cfg(feature = "xlsx_read")]
        "xls" | "xlsx" => {
            let data = std::fs::read(path)
                .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
            detect_excel(&data)
        }
        #[cfg(feature = "pdf_parse")]
        "pdf" => {
            let pages = crate::peripheral::pdf::get_all_pages_text_from_path(
                &path.to_path_buf(),
            )?;
            Ok(detect_from_pdf_text(&pages))
        }
        _ => Ok(FileDetectResult::unknown_bare()),
    }
}

fn detect_from_bytes(
    data: &[u8],
    file_name: &str,
) -> Result<FileDetectResult, SError> {
    let ext = file_extension(file_name).to_lowercase();

    match ext.as_str() {
        "csv" => Ok(detect_csv(data)),
        #[cfg(feature = "xlsx_read")]
        "xls" | "xlsx" => detect_excel(data),
        // For PDF from raw bytes, callers should pre-parse pages and use
        // PdfPages instead.
        _ => Ok(FileDetectResult::unknown_bare()),
    }
}

/// The minimum columns required to identify a CSV as an ACB TX file.
const ACB_CSV_REQUIRED_COLS: &[&str] = &[CsvCol::SECURITY, CsvCol::ACTION];
/// At least one of these date columns must be present.
const ACB_CSV_DATE_COLS: &[&str] =
    &[CsvCol::TRADE_DATE, CsvCol::LEGACY_SETTLEMENT_DATE];

fn detect_csv(data: &[u8]) -> FileDetectResult {
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => return FileDetectResult::unknown_bare(),
    };

    // Read the first line as the header
    let first_line = match text.lines().next() {
        Some(l) => l,
        None => return FileDetectResult::unknown_bare(),
    };

    let headers: HashSet<String> =
        first_line.split(',').map(|h| h.trim().to_lowercase()).collect();

    let mut missing: Vec<&str> = ACB_CSV_REQUIRED_COLS
        .iter()
        .filter(|c| !headers.contains(**c))
        .copied()
        .collect();

    let has_date_col = ACB_CSV_DATE_COLS.iter().any(|c| headers.contains(*c));
    if !has_date_col {
        missing.push(CsvCol::TRADE_DATE);
    }

    if missing.is_empty() {
        return FileDetectResult::ok(FileKind::AcbTxCsv);
    }

    // Check for RBC DI CSV (header is not on the first line due to preamble).
    // Scan the first ~15 lines for the RBC DI header row.
    if detect_rbc_di_csv(text) {
        return FileDetectResult::ok(FileKind::RbcDiCsv);
    }

    let cols =
        missing.iter().map(|c| format!("'{c}'")).collect::<Vec<_>>().join(", ");
    FileDetectResult::unknown(format!(
        "CSV is missing required ACB TX column(s): {cols}"
    ))
}

fn detect_rbc_di_csv(text: &str) -> bool {
    use super::rbc_di::REQUIRED_HEADERS;
    let required: HashSet<String> =
        REQUIRED_HEADERS.iter().map(|h| h.to_lowercase()).collect();

    for line in text.lines().take(15) {
        // Parse as CSV to handle quoted fields
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());
        if let Some(Ok(record)) = rdr.records().next() {
            let cols: HashSet<String> =
                record.iter().map(|c| c.trim().to_lowercase()).collect();
            if required.iter().all(|h| cols.contains(h)) {
                return true;
            }
        }
    }
    false
}

#[cfg(feature = "xlsx_read")]
const ETRADE_BENEFIT_SHEET_NAMES: &[&str] = &["ESPP", "Restricted Stock"];

#[cfg(feature = "xlsx_read")]
fn detect_excel(data: &[u8]) -> Result<FileDetectResult, SError> {
    // Check sheet names first for multi-sheet workbooks (E*TRADE BenefitHistory).
    let sheet_names = crate::peripheral::excel::xl_data_sheet_names(data)?;
    let has_etrade_sheet = ETRADE_BENEFIT_SHEET_NAMES
        .iter()
        .any(|name| sheet_names.iter().any(|s| s == name));
    if has_etrade_sheet {
        return Ok(FileDetectResult::ok(FileKind::EtradeBenefitsExcel));
    }

    let sheet_names = &sheet_names; // reuse from above
    let first_sheet_name =
        sheet_names.first().ok_or_else(|| "Workbook has no sheets".to_string())?;
    let sheet = crate::peripheral::excel::read_xl_data(
        data.to_vec(),
        Some(first_sheet_name),
    )?;

    let mut rows = sheet.rows();
    let first_row = match rows.next() {
        Some(r) => r,
        None => return Ok(FileDetectResult::unknown_bare()),
    };

    let col_names: HashSet<String> = first_row
        .iter()
        .filter_map(|cell| match cell {
            calamine::Data::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect();

    // Check for Questrade columns
    let is_questrade =
        QUESTRADE_REQUIRED_HEADERS.iter().all(|h| col_names.contains(*h));
    if is_questrade {
        return Ok(FileDetectResult::ok(FileKind::QuestradeExcel));
    }

    Ok(FileDetectResult::unknown(
        "Excel file does not match any known broker format".to_string(),
    ))
}

fn detect_from_pdf_text(pages: &[String]) -> FileDetectResult {
    let full_text = pages.join("\n");

    if ETRADE_BENEFIT_PDF_PATTERN.is_match(&full_text) {
        FileDetectResult::ok(FileKind::EtradeBenefitPdf)
    } else if ETRADE_TRADE_CONF_PDF_PATTERN.is_match(&full_text) {
        FileDetectResult::ok(FileKind::EtradeTradeConfirmationPdf)
    } else {
        FileDetectResult::unknown(
            "PDF does not match any known broker format".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(source: FileDetectSource) -> FileDetectResult {
        detect_file_kind(source).unwrap()
    }

    fn assert_kind(result: &FileDetectResult, expected: FileKind) {
        assert_eq!(result.kind, expected);
    }

    fn assert_ok(result: &FileDetectResult, expected: FileKind) {
        assert_kind(result, expected);
        assert!(
            result.warning.is_none(),
            "unexpected warning: {:?}",
            result.warning
        );
    }

    fn assert_unknown_with_warning(result: &FileDetectResult, substring: &str) {
        assert_kind(result, FileKind::Unknown);
        let w = result.warning.as_ref().expect("expected a warning");
        assert!(
            w.contains(substring),
            "warning {w:?} should contain {substring:?}"
        );
    }

    fn assert_unknown_bare(result: &FileDetectResult) {
        assert_kind(result, FileKind::Unknown);
        assert!(
            result.warning.is_none(),
            "unexpected warning: {:?}",
            result.warning
        );
    }

    // -- CSV via Bytes --

    #[test]
    fn test_csv_acb_modern_headers() {
        let csv = "security,trade date,settlement date,action,shares,amount/share\nFOO,2024-01-01,,Buy,10,5.00\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "txs.csv",
        });
        assert_ok(&r, FileKind::AcbTxCsv);
    }

    #[test]
    fn test_csv_acb_legacy_date_header() {
        let csv =
            "security,date,action,shares,amount/share\nFOO,2024-01-01,Buy,10,5.00\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "txs.csv",
        });
        assert_ok(&r, FileKind::AcbTxCsv);
    }

    #[test]
    fn test_csv_acb_case_insensitive() {
        let csv = "Security,Trade Date,Action,Shares\nFOO,2024-01-01,Buy,10\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "txs.csv",
        });
        assert_ok(&r, FileKind::AcbTxCsv);
    }

    #[test]
    fn test_csv_missing_action_warns() {
        let csv = "security,trade date,shares\nFOO,2024-01-01,10\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "txs.csv",
        });
        assert_unknown_with_warning(&r, "'action'");
    }

    #[test]
    fn test_csv_missing_date_warns() {
        let csv = "security,action,shares\nFOO,Buy,10\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "txs.csv",
        });
        assert_unknown_with_warning(&r, "'trade date'");
    }

    #[test]
    fn test_csv_unrelated_csv_warns_all_missing() {
        let csv = "name,age,city\nAlice,30,Vancouver\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "people.csv",
        });
        assert_unknown_with_warning(&r, "'security'");
        assert_unknown_with_warning(&r, "'action'");
        assert_unknown_with_warning(&r, "'trade date'");
    }

    #[test]
    fn test_csv_empty() {
        let r = detect(FileDetectSource::Bytes {
            data: b"",
            file_name: "empty.csv",
        });
        assert_unknown_bare(&r);
    }

    // -- RBC DI CSV detection --

    #[test]
    fn test_csv_rbc_di_with_preamble() {
        let csv = "\
\"Activity Export as of Mar 29, 2026 at 10:51:18 pm ET\"

\"Account: 12345 - RRSP\"

\"Trades this month: 0\"

\"5 Activities\"

\"Date\",\"Activity\",\"Symbol\",\"Symbol Description\",\"Quantity\",\"Price\",\"Settlement Date\",\"Account\",\"Value\",\"Currency\",\"Description\"
\"December 24, 2025\",\"Buy\",\"XEQT\",\"ISHARES\",\"6\",\"40.37\",\"December 29, 2025\",\"12345\",\"-242.22\",\"CAD\",\"desc\"
";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "activity.csv",
        });
        assert_ok(&r, FileKind::RbcDiCsv);
    }

    #[test]
    fn test_csv_rbc_di_header_on_first_line() {
        // If someone strips the preamble, it should still detect as RBC DI
        // (but actually this would look like an unknown CSV since the first-line
        // check for ACB columns runs first and fails, then the RBC scan finds it).
        let csv = "\"Date\",\"Activity\",\"Symbol\",\"Symbol Description\",\"Quantity\",\"Price\",\"Settlement Date\",\"Account\",\"Value\",\"Currency\",\"Description\"\n\"January 3, 2025\",\"Buy\",\"FOO\",\"desc\",\"10\",\"5.00\",\"January 5, 2025\",\"12345\",\"-50\",\"CAD\",\"desc\"\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "activity.csv",
        });
        assert_ok(&r, FileKind::RbcDiCsv);
    }

    // -- Non-CSV extension with CSV content should be Unknown --

    #[test]
    fn test_non_csv_extension_ignored() {
        let csv = "security,trade date,action,shares\nFOO,2024-01-01,Buy,10\n";
        let r = detect(FileDetectSource::Bytes {
            data: csv.as_bytes(),
            file_name: "data.txt",
        });
        assert_unknown_bare(&r);
    }

    // -- PDF pages --

    #[test]
    fn test_pdf_rsu_benefit() {
        let pages = vec![
            "Employee ID: 1234\nSTOCK  PLAN  RELEASE  CONFIRMATION\nRelease Date 2024-01-15".to_string(),
        ];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeBenefitPdf);
    }

    #[test]
    fn test_pdf_eso_benefit() {
        let pages =
            vec!["Employee ID: 5678\nSTOCK PLAN EXERCISE CONFIRMATION\nsome data"
                .to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeBenefitPdf);
    }

    #[test]
    fn test_pdf_espp_benefit() {
        let pages =
            vec!["Some header\nPlan 2014\nESPP purchase details".to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeBenefitPdf);
    }

    #[test]
    fn test_pdf_espp_benefit_esp2() {
        let pages =
            vec!["Some header\nPlan ESP2\nESPP purchase details".to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeBenefitPdf);
    }

    #[test]
    fn test_pdf_pre_ms_trade_confirmation() {
        let pages =
            vec!["Employee ID: 1111\nTRADE CONFIRMATION\nPage 1 of 2".to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeTradeConfirmationPdf);
    }

    #[test]
    fn test_pdf_post_ms_trade_confirmation() {
        let pages =
            vec!["Morgan Stanley\nThis transaction is confirmed\nDetails follow"
                .to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeTradeConfirmationPdf);
    }

    #[test]
    fn test_pdf_unknown_has_warning() {
        let pages =
            vec!["Just some random PDF content\nNothing broker-related here"
                .to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_unknown_with_warning(&r, "PDF");
    }

    #[test]
    fn test_pdf_empty_pages_has_warning() {
        let pages: Vec<String> = vec![];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_unknown_with_warning(&r, "PDF");
    }

    #[test]
    fn test_pdf_multi_page_match() {
        let pages = vec![
            "Page 1: nothing useful".to_string(),
            "Page 2: STOCK PLAN RELEASE CONFIRMATION".to_string(),
        ];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeBenefitPdf);
    }

    // -- Benefit takes priority over trade confirmation --

    #[test]
    fn test_pdf_benefit_priority_over_trade_conf() {
        let pages =
            vec!["STOCK PLAN RELEASE CONFIRMATION\nTRADE CONFIRMATION".to_string()];
        let r = detect(FileDetectSource::PdfPages(&pages));
        assert_ok(&r, FileKind::EtradeBenefitPdf);
    }

    // -- Excel detection --

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_xlsx_etrade_benefits_espp_sheet() {
        use rust_xlsxwriter::Workbook;
        let mut wb = Workbook::new();
        wb.add_worksheet().set_name("ESPP").unwrap();
        let data = wb.save_to_buffer().unwrap();
        let r = detect(FileDetectSource::Bytes {
            data: &data,
            file_name: "BenefitHistory.xlsx",
        });
        assert_ok(&r, FileKind::EtradeBenefitsExcel);
    }

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_xlsx_etrade_benefits_restricted_stock_sheet() {
        use rust_xlsxwriter::Workbook;
        let mut wb = Workbook::new();
        wb.add_worksheet().set_name("Restricted Stock").unwrap();
        let data = wb.save_to_buffer().unwrap();
        let r = detect(FileDetectSource::Bytes {
            data: &data,
            file_name: "BenefitHistory.xlsx",
        });
        assert_ok(&r, FileKind::EtradeBenefitsExcel);
    }

    #[cfg(feature = "xlsx_write")]
    #[test]
    fn test_xlsx_unknown_with_headers() {
        use rust_xlsxwriter::Workbook;
        let mut wb = Workbook::new();
        let sheet = wb.add_worksheet().set_name("Data").unwrap();
        sheet.write(0, 0, "Name").unwrap();
        sheet.write(0, 1, "Value").unwrap();
        let data = wb.save_to_buffer().unwrap();
        let r = detect(FileDetectSource::Bytes {
            data: &data,
            file_name: "unknown.xlsx",
        });
        assert_unknown_with_warning(&r, "does not match");
    }
}
