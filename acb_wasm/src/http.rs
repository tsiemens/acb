use acb::util::basic::SError;
use acb::util::http::HttpRequester;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Implements HttpRequester for a restrictive browser environment,
/// so that we can make arbitrary http requests to third-party
/// sites (eg. using the BoC rates API).
pub struct CorsEnabledHttpRequester {}

impl CorsEnabledHttpRequester {
    pub fn new_boxed() -> Box<Self> {
        Box::new(Self {})
    }
}

#[async_trait::async_trait(?Send)]
impl HttpRequester for CorsEnabledHttpRequester {
    async fn get(&self, url: &str) -> Result<String, SError> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| format!("Error creating request for {url}: {:?}", e))?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("Fetch error for {url}: {:?}", e))?;

        // `resp_value` is a `Response` object.
        assert!(resp_value.is_instance_of::<Response>());
        let resp: Response = resp_value.dyn_into().unwrap();

        // Convert the response to text
        let text_promise = resp.text().unwrap();
        let text_js_future = JsFuture::from(text_promise);
        let text_js_value = text_js_future
            .await
            .map_err(|e| format!("Error getting body text for {url}: {:?}", e))?;

        // Convert JsValue to String
        let text: String = text_js_value.as_string().unwrap();
        Ok(text)
    }
}
