use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use acb::{
    fx::{
        io::{InMemoryRatesCache, JsonRemoteRateLoader, RateLoader},
        DailyRate,
    },
    util::{basic::SError, rw::WriteHandle},
};

const FORCE_DOWNLOAD_RATES: bool = false;

// -- Serializable interchange types for browser-cached FX rates --

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableDailyRate {
    pub date: String,
    pub rate: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializableYearRates {
    pub year: u32,
    pub rates: Vec<SerializableDailyRate>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RatesCacheData {
    pub years: Vec<SerializableYearRates>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RatesCacheUpdate {
    pub years: Vec<SerializableYearRates>,
}

fn daily_rates_to_serializable(rates: &[DailyRate]) -> Vec<SerializableDailyRate> {
    rates
        .iter()
        .map(|r| SerializableDailyRate {
            date: r.date.to_string(),
            rate: r.foreign_to_local_rate.to_string(),
        })
        .collect()
}

fn serializable_to_daily_rates(
    rates: &[SerializableDailyRate],
) -> Result<Vec<DailyRate>, SError> {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    rates
        .iter()
        .map(|r| {
            let date = acb::util::date::parse_standard_date(&r.date)
                .map_err(|e| format!("Invalid date '{}': {}", r.date, e))?;
            let rate = Decimal::from_str(&r.rate)
                .map_err(|e| format!("Invalid rate '{}': {}", r.rate, e))?;
            Ok(DailyRate {
                date,
                foreign_to_local_rate: rate,
            })
        })
        .collect()
}

fn deserialize_initial_rates(
    data: &RatesCacheData,
) -> Result<HashMap<u32, Vec<DailyRate>>, SError> {
    let mut map = HashMap::new();
    for yr in &data.years {
        let rates = serializable_to_daily_rates(&yr.rates)?;
        map.insert(yr.year, rates);
    }
    Ok(map)
}

pub fn make_rate_loader(
    err_write_handle: WriteHandle,
    initial_rates: Option<&RatesCacheData>,
) -> Result<RateLoader, SError> {
    let cache = if let Some(data) = initial_rates {
        InMemoryRatesCache::new_with_rates(deserialize_initial_rates(data)?)
    } else {
        InMemoryRatesCache::new()
    };

    Ok(RateLoader::new(
        FORCE_DOWNLOAD_RATES,
        Box::new(cache),
        JsonRemoteRateLoader::new_boxed(
            crate::http::CorsEnabledHttpRequester::new_boxed(),
        ),
        err_write_handle,
    ))
}

pub fn build_rates_cache_update(rate_loader: &mut RateLoader) -> RatesCacheUpdate {
    let fresh_years: Vec<u32> =
        rate_loader.fresh_loaded_years().iter().copied().collect();
    let mut years = Vec::new();
    for year in &fresh_years {
        if let Ok(Some(rates)) = rate_loader.cache.get_usd_cad_rates(*year) {
            years.push(SerializableYearRates {
                year: *year,
                rates: daily_rates_to_serializable(&rates),
            });
        }
    }
    years.sort_by_key(|y| y.year);
    RatesCacheUpdate { years }
}
