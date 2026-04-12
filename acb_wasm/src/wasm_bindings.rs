use regex::Regex;
use serde::Serialize;

use acb::app::config::AcbConfig;
use acb::util::rw::DescribedReader;
use wasm_bindgen::prelude::*;

use crate::app_shim;
use crate::wasm_rates_loader;

#[derive(serde::Serialize)]
struct ConfigParseResult {
    config: AcbConfig,
    warnings: Vec<String>,
}

/// Parse and validate a config JSON string.
/// Returns the validated config as a JS object, or an error string.
#[wasm_bindgen]
pub fn parse_config(json: &str) -> Result<JsValue, JsValue> {
    let config = AcbConfig::from_json(json).map_err(|e| JsValue::from_str(&e))?;
    let warnings = AcbConfig::validate_warnings(json);
    let result = ConfigParseResult { config, warnings };
    let serializer =
        serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    Ok(result.serialize(&serializer)?)
}

/// Return a default (empty) AcbConfig as a JS object.
#[wasm_bindgen]
pub fn make_default_config() -> Result<JsValue, JsValue> {
    let config = AcbConfig::new();
    let serializer =
        serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    Ok(config.serialize(&serializer)?)
}

/// Serialize a config object to pretty-printed JSON.
#[wasm_bindgen]
pub fn serialize_config(config_js: JsValue) -> Result<String, JsValue> {
    let config: AcbConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {e}")))?;
    config.to_json().map_err(|e| JsValue::from_str(&e))
}

fn parse_optional_config(
    config_json: Option<String>,
) -> Result<Option<AcbConfig>, JsValue> {
    match config_json {
        Some(json) if !json.is_empty() => {
            let config =
                AcbConfig::from_json(&json).map_err(|e| JsValue::from_str(&e))?;
            Ok(Some(config))
        }
        _ => Ok(None),
    }
}

/// Convert a raw Excel workbook (as bytes) to ACB-format CSV text.
///
/// `sheet_name` selects which sheet to parse; pass `None`/`undefined` from JS
/// to auto-select the only sheet (errors if there is more than one).
///
/// Returns an `XlConvertResult` with `csvText` and `nonFatalErrors` fields,
/// or an error string on fatal failure.
#[wasm_bindgen]
pub fn convert_xl_to_csv(
    data: Vec<u8>,
    sheet_name: Option<String>,
    no_fx: bool,
    config_json: Option<String>,
) -> Result<JsValue, JsValue> {
    use acb::peripheral::excel::XlSource;
    use acb::peripheral::tx_export_convert_impl::{convert_xl_txs, BrokerArg};
    use acb::portfolio::io::tx_csv::write_txs_to_csv;

    let config = parse_optional_config(config_json)?;

    let convert_result = convert_xl_txs(
        XlSource::Data(data),
        &BrokerArg::Questrade,
        sheet_name.as_deref(),
        Some(Regex::new(r".").unwrap()), // account_filter
        None,                            // security_filter
        no_fx,
        false, // no_sort (i.e. sort by default)
        None,  // usd_exchange_rate
        config.as_ref(),
    )
    .map_err(|e| JsValue::from_str(&e))?;

    let mut buf: Vec<u8> = Vec::new();
    write_txs_to_csv(&convert_result.csv_txs, &mut buf)
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
    let csv_text =
        String::from_utf8(buf).map_err(|e| JsValue::from_str(&format!("{e}")))?;

    let result = app_shim::XlConvertResult {
        csv_text,
        non_fatal_errors: convert_result.non_fatal_errors,
    };
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

/// Convert an RBC Direct Investing CSV export to ACB-format CSV text.
///
/// Returns a `CsvBrokerConvertResult` with `csvText`, `nonFatalErrors`, and
/// `warnings` fields, or an error string on fatal failure.
#[wasm_bindgen]
pub fn convert_rbc_di_csv(
    data: Vec<u8>,
    no_fx: bool,
    config_json: Option<String>,
) -> Result<JsValue, JsValue> {
    use acb::peripheral::tx_export_convert_impl::convert_csv_broker_txs;
    use acb::portfolio::io::tx_csv::write_txs_to_csv;

    let config = parse_optional_config(config_json)?;

    let result = convert_csv_broker_txs(
        &data,
        None,                            // fpath
        Some(Regex::new(r".").unwrap()), // account_filter
        None,                            // security_filter
        no_fx,
        false, // no_sort
        None,  // usd_exchange_rate
        config.as_ref(),
    )
    .map_err(|e| JsValue::from_str(&e))?;

    let mut buf: Vec<u8> = Vec::new();
    write_txs_to_csv(&result.csv_txs, &mut buf)
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
    let csv_text =
        String::from_utf8(buf).map_err(|e| JsValue::from_str(&format!("{e}")))?;

    let convert_result = app_shim::CsvBrokerConvertResult {
        csv_text,
        non_fatal_errors: result.non_fatal_errors,
        warnings: result.warnings,
    };
    Ok(serde_wasm_bindgen::to_value(&convert_result)?)
}

#[wasm_bindgen]
pub fn get_acb_version() -> String {
    acb::app::ACB_APP_VERSION.to_string()
}

fn file_detect_result_to_js(
    result: acb::peripheral::broker::FileDetectResult,
) -> JsValue {
    use acb::peripheral::broker::FileKind;

    let kind_str = match result.kind {
        FileKind::AcbTxCsv => "AcbTxCsv",
        FileKind::QuestradeExcel => "QuestradeExcel",
        FileKind::RbcDiCsv => "RbcDiCsv",
        FileKind::EtradeTradeConfirmationPdf => "EtradeTradeConfirmationPdf",
        FileKind::EtradeBenefitPdf => "EtradeBenefitPdf",
        FileKind::EtradeBenefitsExcel => "EtradeBenefitsExcel",
        FileKind::AcbConfigJson => "AcbConfigJson",
        FileKind::Unknown => "Unknown",
    };

    let js_obj = web_sys::js_sys::Object::new();
    web_sys::js_sys::Reflect::set(
        &js_obj,
        &JsValue::from_str("kind"),
        &JsValue::from_str(kind_str),
    )
    .unwrap();
    if let Some(warning) = result.warning {
        web_sys::js_sys::Reflect::set(
            &js_obj,
            &JsValue::from_str("warning"),
            &JsValue::from_str(&warning),
        )
        .unwrap();
    }
    js_obj.into()
}

fn run_file_detect(source: acb::peripheral::broker::FileDetectSource) -> JsValue {
    use acb::peripheral::broker::{
        detect_file_kind as detect, FileDetectResult, FileKind,
    };

    let result = detect(source).unwrap_or(FileDetectResult {
        kind: FileKind::Unknown,
        warning: None,
    });
    file_detect_result_to_js(result)
}

/// Detect the kind of broker/ACB file from raw bytes and a file name.
///
/// Returns a `{ kind: string, warning?: string }` object.
/// `kind` is a tag like "AcbTxCsv", "QuestradeExcel", "Unknown", etc.
/// `warning` is an optional hint explaining why detection returned Unknown.
#[wasm_bindgen]
pub fn detect_file_kind(data: &[u8], file_name: &str) -> JsValue {
    use acb::peripheral::broker::FileDetectSource;
    run_file_detect(FileDetectSource::Bytes { data, file_name })
}

/// Detect the kind of PDF from pre-extracted page texts.
///
/// Returns a `{ kind: string, warning?: string }` object, same shape as
/// `detect_file_kind`.
#[wasm_bindgen]
pub fn detect_file_kind_from_pdf_pages(pages: Vec<String>) -> JsValue {
    use acb::peripheral::broker::FileDetectSource;
    run_file_detect(FileDetectSource::PdfPages(&pages))
}

/// Convert Uint8Array JS array items into Vec<(Vec<u8>, String)>
/// (Pairs of (xlsx data, xlsx name))
fn unpack_xlsx_args(
    xlsx_datas: Vec<web_sys::js_sys::Uint8Array>,
    xlsx_names: Vec<String>,
) -> Result<Vec<(Vec<u8>, String)>, JsValue> {
    if xlsx_datas.len() != xlsx_names.len() {
        return Err(JsValue::from_str(
            "xlsx_datas and xlsx_names must have the same length",
        ));
    }
    let mut files: Vec<(Vec<u8>, String)> = xlsx_datas
        .into_iter()
        .zip(xlsx_names)
        .map(|(data, name)| (data.to_vec(), name))
        .collect();
    files.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(files)
}

/// Convert E*TRADE PDF texts to ACB-format CSV.
///
/// `pdf_texts` contains the full text of each PDF, `file_names` has
/// corresponding file names (used for error context).
///
/// Returns an `EtradeConvertResult` with `csvText`, `warnings`, and
/// `nonFatalErrors` fields, or an error string on fatal failure.
#[wasm_bindgen]
pub fn convert_etrade_pdfs_to_csv(
    pdf_texts: Vec<String>,
    file_names: Vec<String>,
    xlsx_datas: Vec<web_sys::js_sys::Uint8Array>,
    xlsx_names: Vec<String>,
    generate_fx: bool,
    no_sell_to_cover_pair: bool,
    year: Option<i32>,
    config_json: Option<String>,
) -> Result<JsValue, JsValue> {
    use acb::peripheral::etrade_plan_pdf_tx_extract_impl::convert_etrade_file_data;

    if pdf_texts.len() != file_names.len() {
        return Err(JsValue::from_str(
            "pdf_texts and file_names must have the same length",
        ));
    }

    let config = parse_optional_config(config_json)?;

    let mut pairs: Vec<(String, String)> =
        pdf_texts.into_iter().zip(file_names).collect();
    pairs.sort_by(|a, b| a.1.cmp(&b.1));

    let xlsx_files = unpack_xlsx_args(xlsx_datas, xlsx_names)?;

    let result = convert_etrade_file_data(
        &pairs,
        &xlsx_files,
        generate_fx,
        no_sell_to_cover_pair,
        year,
        config.as_ref(),
    )
    .map_err(|errs| JsValue::from_str(&errs.join("\n")))?;

    let convert_result = app_shim::EtradeConvertResult {
        csv_text: result.csv_text,
        warnings: result.warnings,
    };
    Ok(serde_wasm_bindgen::to_value(&convert_result)?)
}

/// Extract raw E*TRADE PDF/xlsx data without harmonizing benefits and trades.
///
/// Returns an object with `benefitsTable` and `tradesTable` RenderTable fields.
#[wasm_bindgen]
pub fn extract_etrade_pdf_data(
    pdf_texts: Vec<String>,
    pdf_names: Vec<String>,
    xlsx_datas: Vec<web_sys::js_sys::Uint8Array>,
    xlsx_names: Vec<String>,
    year: Option<i32>,
) -> Result<JsValue, JsValue> {
    use acb::peripheral::etrade_plan_pdf_tx_extract_impl::extract_etrade_file_data_to_render_tables;

    if pdf_texts.len() != pdf_names.len() {
        return Err(JsValue::from_str(
            "pdf_texts and file_names must have the same length",
        ));
    }

    let mut pairs: Vec<(String, String)> =
        pdf_texts.into_iter().zip(pdf_names).collect();
    pairs.sort_by(|a, b| a.1.cmp(&b.1));

    let xlsx_files = unpack_xlsx_args(xlsx_datas, xlsx_names)?;

    let result =
        extract_etrade_file_data_to_render_tables(&pairs, &xlsx_files, year)
            .map_err(|errs| JsValue::from_str(&errs.join("\n")))?;

    let extract_result = app_shim::EtradeExtractResult {
        benefits_table: app_shim::SerializableRenderTable(result.benefits_table),
        trades_table: app_shim::SerializableRenderTable(result.trades_table),
    };
    Ok(serde_wasm_bindgen::to_value(&extract_result)?)
}

/// Extract unique account numbers from broker files.
///
/// `file_datas` and `file_names` are parallel arrays of non-PDF file bytes
/// and names (xlsx, csv, etc.).
/// `pdf_texts` and `pdf_names` are parallel arrays of pre-extracted PDF
/// full-text strings and their file names.
///
/// Returns an `AccountExtractionResult` with `accounts` and `warnings`.
#[wasm_bindgen]
pub fn extract_account_numbers(
    file_datas: Vec<web_sys::js_sys::Uint8Array>,
    file_names: Vec<String>,
    pdf_texts: Vec<String>,
    pdf_names: Vec<String>,
) -> Result<JsValue, JsValue> {
    use acb::peripheral::broker::extract_accounts_from_files;

    if file_datas.len() != file_names.len() {
        return Err(JsValue::from_str(
            "file_datas and file_names must have the same length",
        ));
    }
    if pdf_texts.len() != pdf_names.len() {
        return Err(JsValue::from_str(
            "pdf_texts and pdf_names must have the same length",
        ));
    }

    // Convert Uint8Array items to Vec<u8>
    let file_bytes: Vec<Vec<u8>> =
        file_datas.into_iter().map(|d| d.to_vec()).collect();
    let files: Vec<(&[u8], &str)> = file_bytes
        .iter()
        .zip(file_names.iter())
        .map(|(data, name)| (data.as_slice(), name.as_str()))
        .collect();

    // Split each pdf text into pages (single page per entry since JS
    // already joins pages).  The Rust side expects &[String] per file.
    let pdf_page_vecs: Vec<Vec<String>> =
        pdf_texts.into_iter().map(|t| vec![t]).collect();
    let pdf_page_texts: Vec<(&[String], &str)> = pdf_page_vecs
        .iter()
        .zip(pdf_names.iter())
        .map(|(pages, name)| (pages.as_slice(), name.as_str()))
        .collect();

    let (accounts, warnings) = extract_accounts_from_files(&files, &pdf_page_texts);
    let result = app_shim::to_account_extraction_result(accounts, warnings);
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

fn parse_initial_rates(
    initial_rates_cache: JsValue,
) -> Result<Option<wasm_rates_loader::RatesCacheData>, JsValue> {
    if initial_rates_cache.is_undefined() || initial_rates_cache.is_null() {
        Ok(None)
    } else {
        Ok(Some(
            serde_wasm_bindgen::from_value(initial_rates_cache).map_err(|e| {
                JsValue::from_str(&format!(
                    "Failed to deserialize rates cache: {}",
                    e
                ))
            })?,
        ))
    }
}

fn get_csv_readers(
    file_descs: Vec<String>,
    file_contents: Vec<String>,
) -> Result<Vec<DescribedReader>, JsValue> {
    if file_descs.len() != file_contents.len() {
        return Err("".to_string().into());
    }

    let mut csv_readers = Vec::<DescribedReader>::new();
    for (desc, content) in file_descs.into_iter().zip(file_contents) {
        csv_readers.push(DescribedReader::from_string(desc, content));
    }

    Ok(csv_readers)
}

#[wasm_bindgen]
pub async fn run_acb(
    file_descs: Vec<String>,
    file_contents: Vec<String>,
    render_full_values: bool,
    export_mode: bool,
    initial_rates_cache: JsValue,
    config_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let csv_readers = get_csv_readers(file_descs, file_contents)?;
    let initial_rates = parse_initial_rates(initial_rates_cache)?;
    let config = parse_optional_config(config_json)?;

    if export_mode {
        let result = app_shim::run_acb_app_for_export(
            csv_readers,
            render_full_values,
            initial_rates.as_ref(),
            config.as_ref(),
        )
        .await
        .map_err(|e| JsValue::from_str(&e))?;
        return Ok(serde_wasm_bindgen::to_value(&result)?);
    }

    let result = app_shim::run_acb_app(
        csv_readers,
        render_full_values,
        initial_rates.as_ref(),
        config.as_ref(),
    )
    .await
    .map_err(|e| JsValue::from_str(&e))?;

    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[wasm_bindgen]
pub async fn run_acb_summary(
    latest_date: web_sys::js_sys::Date,
    file_descs: Vec<String>,
    file_contents: Vec<String>,
    split_annual_summary_gains: bool,
    render_full_values: bool,
    initial_rates_cache: JsValue,
    config_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let csv_readers = get_csv_readers(file_descs, file_contents)?;
    let initial_rates = parse_initial_rates(initial_rates_cache)?;
    let config = parse_optional_config(config_json)?;

    let latest_date_rs = acb::util::date::from_date_ints(
        latest_date.get_full_year() as i32,
        (latest_date.get_month() + 1) as u8,
        latest_date.get_date() as u8,
    )
    .map_err(|e| {
        JsValue::from_str(&format!("Error converting date {:?}: {}", latest_date, e))
    })?;

    let result = app_shim::run_acb_app_summary(
        latest_date_rs,
        csv_readers,
        split_annual_summary_gains,
        render_full_values,
        initial_rates.as_ref(),
        config.as_ref(),
    )
    .await
    .map_err(|e| JsValue::from_str(&e))?;

    Ok(serde_wasm_bindgen::to_value(&result)?)
}
