use regex::Regex;

use acb::util::rw::DescribedReader;
use wasm_bindgen::prelude::*;

pub mod app_shim;
pub mod http;

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
) -> Result<JsValue, JsValue> {
    use acb::peripheral::excel::XlSource;
    use acb::peripheral::tx_export_convert_impl::{convert_xl_txs, BrokerArg};
    use acb::portfolio::io::tx_csv::write_txs_to_csv;

    let (csv_txs, non_fatal_errors) = convert_xl_txs(
        XlSource::Data(data),
        &BrokerArg::Questrade,
        sheet_name.as_deref(),
        Some(Regex::new(r".").unwrap()),  // account_filter
        None,  // security_filter
        false, // no_fx
        false, // no_sort (i.e. sort by default)
        None,  // usd_exchange_rate
    )
    .map_err(|e| JsValue::from_str(&e))?;

    let mut buf: Vec<u8> = Vec::new();
    write_txs_to_csv(&csv_txs, &mut buf)
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
    let csv_text =
        String::from_utf8(buf).map_err(|e| JsValue::from_str(&format!("{e}")))?;

    let result = app_shim::XlConvertResult { csv_text, non_fatal_errors };
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[wasm_bindgen]
pub fn get_acb_version() -> String {
    acb::app::ACB_APP_VERSION.to_string()
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
) -> Result<JsValue, JsValue> {
    let csv_readers = get_csv_readers(file_descs, file_contents)?;

    if export_mode {
        let result =
            app_shim::run_acb_app_for_export(csv_readers, render_full_values)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
        return Ok(serde_wasm_bindgen::to_value(&result)?);
    }

    let result =
        app_shim::run_acb_app(csv_readers, render_full_values)
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
) -> Result<JsValue, JsValue> {
    let csv_readers = get_csv_readers(file_descs, file_contents)?;

    let latest_date_rs = acb::util::date::from_date_ints(
        latest_date.get_full_year() as i32,
        (latest_date.get_month() + 1) as u8,
        latest_date.get_date() as u8).
        map_err(|e| JsValue::from_str(
            &format!("Error converting date {:?}: {}", latest_date, e)))?;

    let result =
        app_shim::run_acb_app_summary(
            latest_date_rs, csv_readers,
            split_annual_summary_gains, render_full_values)
            .await
            .map_err(|e| JsValue::from_str(&e))?;

    Ok(serde_wasm_bindgen::to_value(&result)?)
}
