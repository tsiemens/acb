use acb::app::input_parse::parse_initial_status;
use acb::util::rw::DescribedReader;
use wasm_bindgen::prelude::*;

pub mod app_shim;
pub mod http;

#[wasm_bindgen]
pub fn get_acb_version() -> String {
    acb::app::ACB_APP_VERSION.to_string()
}

#[wasm_bindgen]
pub async fn run_acb(
    file_descs: Vec<String>,
    file_contents: Vec<String>,
    initial_security_states: Vec<String>,
    render_full_values: bool,
    ) -> Result<JsValue, JsValue> {

    if file_descs.len() != file_contents.len() {
        return Err("".to_string().into());
    }

    let mut csv_readers = Vec::<DescribedReader>::new();
    for (desc, content) in file_descs.into_iter().zip(file_contents) {
        csv_readers.push(DescribedReader::from_string(desc, content));
    }

    let all_init_status = parse_initial_status(&initial_security_states)
        .map_err(|e| JsValue::from_str(&e))?;

    let result = app_shim::run_acb_app(
            csv_readers, all_init_status, render_full_values).await
        .map_err(|e| JsValue::from_str(&e))?;

    Ok(serde_wasm_bindgen::to_value(&result)?)
}