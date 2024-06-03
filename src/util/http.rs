use super::basic::SError;

/// Used to permit multiple http get implementation/libraries.
/// Depending on if this is being run in the standalone CLI app or in wasm/
/// a browser, we need multiple http libraries right now because they don't
/// all quite work in all cases, particularly because of inconsistent CORS mode
/// support. This can also be used to avoid compilation conflicts for library
/// incompatabilities, like surf/stdweb and wasm-bindgen.
///
/// async_trait is required to be able to instantiate a Box<dyn HttpRequester>
/// of this. This is because rust doesn't have full native support for returning Futures
/// from traits right now. This is marked ?Sync (not sync) because compiling against
/// wasm will break otherwise, since the underlying core types are not Sync/Send in
/// that mode. We don't actually need this to be thread-safe, so it's not an issue for now.
/// See https://smallcultfollowing.com/babysteps/blog/2019/10/26/async-fn-in-traits-are-hard/
///
/// Also note that we're doing this rather than simply passing around a function pointer
/// because you aren't allowed to have raw async function pointers.
#[async_trait::async_trait(?Send)]
pub trait HttpRequester {
    async fn get(&self, url: &str) -> Result<String, SError>;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod standalone {
    use crate::util::basic::SError;

    use super::HttpRequester;

    pub struct StandaloneAppRequester;

    impl StandaloneAppRequester {
        pub fn new() -> StandaloneAppRequester {
            StandaloneAppRequester{}
        }

        pub fn new_boxed() -> Box<StandaloneAppRequester> {
            Box::new(StandaloneAppRequester::new())
        }
    }

    #[async_trait::async_trait(?Send)]
    impl HttpRequester for StandaloneAppRequester {
        async fn get(&self, url: &str) -> Result<String, SError> {
            // Use surf, because it has no dependence on tokio, so
            // we can more simply run this under async_std::block_on
            // rather than the entirety of main needing to use tokio runtime.
            //
            // Though realistically, we could also use reqwest::blocking, since
            // this only _has_ to be actually async for wasm, which is using web-sys.
            let body_text = surf::get(url).recv_string().await
                .map_err(|e| format!("{}", e))?;
            Ok(body_text)
        }
    }
}

#[cfg(test)]
pub mod testlib {
    use crate::util::basic::SError;

    use super::HttpRequester;

    pub struct UnusableHttpRequester {}

    impl UnusableHttpRequester {
        pub fn new_boxed() -> Box<Self> {
            Box::new(Self{})
        }
    }

    #[async_trait::async_trait(?Send)]
    impl HttpRequester for UnusableHttpRequester {
        async fn get(&self, _url: &str) -> Result<String, SError> {
            panic!();
        }
    }
}