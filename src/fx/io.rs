mod rate_loader;
mod remote_rate_loader;
mod rates_cache;

pub type Error = String;

// Exports
pub use self::rate_loader::*;
pub use self::remote_rate_loader::*;
pub use self::rates_cache::*;