use acb::util::rw::DescribedReader;
use wasm_bindgen::prelude::*;

pub mod app_shim;
pub mod http;

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
