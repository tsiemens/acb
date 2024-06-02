pub mod model;
pub mod text;

#[cfg(not(target_arch = "wasm32"))]
pub mod csv;