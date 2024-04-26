mod remote_rate_loader;

use std::collections::{HashMap, HashSet};

use time::Date;

use crate::log::WriteHandle;
use crate::fx::DailyRate;

pub use self::remote_rate_loader::*;

// TODO move this.
pub struct RateLoader {
    pub year_rates: HashMap<u32, HashMap<Date, DailyRate>>,
    pub force_download: bool,
    // pub cache: RatesCache,
    fresh_loaded_years: HashSet<u32>,
    err_stream: WriteHandle,
    pub remote_loader: dyn RemoteRateLoader,
}
