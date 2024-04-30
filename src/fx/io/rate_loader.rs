use std::collections::{HashMap, HashSet};
use std::io::Write;

use rust_decimal::{prelude::Zero, Decimal};
use time::{Date, Duration, Month};

use crate::log::WriteHandle;
use crate::{fx::DailyRate, util::date::today_local};

use crate::fx::io::RemoteRateLoader;

use super::{Error, RatesCache};

// Overall utility for loading rates (both remotely and from cache).
pub struct RateLoader {
    pub year_rates: HashMap<u32, HashMap<Date, DailyRate>>,
    pub force_download: bool,
    pub cache: Box<dyn RatesCache>,
    fresh_loaded_years: HashSet<u32>,
    err_stream: WriteHandle,
    pub remote_loader: Box<dyn RemoteRateLoader>,
}

impl RateLoader {
    pub fn new() -> RateLoader {
        todo!();
        // RateLoader{}
    }
}

// Fills in gaps in daily rates (for a single year) with zero
// If today is in the same year as the rates, will fill up to yesterday, with the
// assumption that today's rate wouldn't yet be published.
//
// The purpose of this is so that we can differentiate which dates in the cache
// have been previously looked up vs which ones were left empty because they're still
// in the future.
fn fill_in_unknown_day_rates(rates: &Vec<DailyRate>, year: u32) -> Vec<DailyRate> {
    let mut filled_rates: Vec<DailyRate> = Vec::new();
    // Reserve for weekends, which won't be in the rates
    filled_rates.reserve(365);

    let mut date_to_fill = Date::from_calendar_date(
        year as i32, Month::January, 1).unwrap();
    for rate in rates {
        while date_to_fill < rate.date {
            filled_rates.push(DailyRate{date: date_to_fill,
                                        foreign_to_local_rate: Decimal::zero()});
            date_to_fill = date_to_fill.saturating_add(Duration::days(1));
        }
        filled_rates.push(rate.clone());
        date_to_fill = date_to_fill.saturating_add(Duration::days(1));
    }

    let today = today_local();
    while date_to_fill < today && date_to_fill.year() == (year as i32) {
        filled_rates.push(DailyRate{date: date_to_fill,
                                    foreign_to_local_rate: Decimal::zero()});
        date_to_fill = date_to_fill.saturating_add(Duration::days(1));
    }

    filled_rates.shrink_to_fit();
    filled_rates
}

fn make_date_to_rate_map(rates: &Vec<DailyRate>) -> HashMap<Date, DailyRate> {
    let mut map = HashMap::new();
    for rate in rates {
        map.insert(rate.date, rate.clone());
    }
    map
}

impl RateLoader {
    pub fn get_effective_usd_cad_rate(&mut self, trade_date: Date) -> Result<DailyRate, Error> {
        let fmt_err = |e| {
            format!("Unable to retrieve exchange rate for {}: {}", trade_date, e) };
        match self.get_exact_usd_cad_rate(trade_date) {
            Ok(rate_opt) => match rate_opt {
                Some(rate) => Ok(rate),
                None => self.find_usd_cad_preceding_relevant_spot_rate(trade_date)
                    .map_err(fmt_err),
            },
            Err(e) => Err(fmt_err(e)),
        }
    }

    fn get_exact_usd_cad_rate(&mut self, trade_date: Date) -> Result<Option<DailyRate>, Error> {
        let year = trade_date.year() as u32;

        if !self.year_rates.contains_key(&year) {
            let rates = self.fetch_usd_cad_rates_for_date_year(&trade_date)?;
            self.year_rates.insert(year, rates);
        }
        let year_rates = self.year_rates.get(&year).unwrap();
        if let Some(rate) = year_rates.get(&trade_date) {
            if rate.foreign_to_local_rate.is_zero() {
                Ok(None)
            } else {
                Ok(Some(rate.clone()))
            }
        } else {
            let today = today_local();
            if trade_date == today || trade_date > today {
                // There is no rate available for today yet, so error out.
                // The user must manually provide a rate in this scenario.
                return Err(format!(concat!(
                    "No USD/CAD exchange rate is available for {} yet. Either explicitly add to ",
                    "CSV file or modify the exchange rates cache file in ~/.acb/. ",
                    "If today is a bank holiday, use rate for preceding business day."),
                    trade_date));
            }
            // There is no rate for this exact date, but it is for a date in the past,
            // so the caller can try a previous date for the relevant rate. (ie. we are
            // not in an error scenario yet).
            Ok(None)
        }
    }

    // Loads exchange rates for the year of target_date from cache or from remote web API.
    //
    // Will use the cache if we are not force downloading, if we already downloaded
    // in this process run, or if `target_date` has a defined value in the cache
    // (even if it is defined as zero).
    // Using `target_date` for cache invalidation allows us to avoid invalidating the cache if
    // there are no new transactions.
    fn fetch_usd_cad_rates_for_date_year(&mut self, target_date: &Date) -> Result<HashMap<Date, DailyRate>, Error> {
        let year = target_date.year() as u32;
        if !self.force_download {
            // Try the cache
            let rates_are_fresh = self.fresh_loaded_years.contains(&year);
            let cache_res = self.cache.get_usd_cad_rates(year);
            if let Err(e) = cache_res {
                if rates_are_fresh {
                    // We already loaded this year from remote during this process.
                    // Something is wrong if we tried to access it via the cache again and it
                    // failed (since we are not allowed to make the same request again).
                    return Err(e);
                }
                // This is non-fatal, as we can just do a server lookup.
                let _ = writeln!(self.err_stream, "Could not load cached exchange rates: {}", e);
            } else {
                match cache_res.unwrap() {
                    Some(rates) => {
                        let rates_map = make_date_to_rate_map(&rates);
                        if rates_are_fresh {
                            return Ok(rates_map);
                        } else {
                            // Check for cache invalidation.
                            if rates_map.contains_key(target_date) {
                                return Ok(rates_map);
                            }
                        }
                    },
                    None => {
                        if rates_are_fresh {
                            return Err(format!("Did not find rates for {} in cache after they were downloaded", year));
                        }
                    },
                }
            }
        }

        self.get_remote_usd_cad_rates(year)
            .map(|r| { make_date_to_rate_map(&r) })
    }

    fn get_remote_usd_cad_rates(&mut self, year: u32) -> Result<Vec<DailyRate>, Error> {
        let res = self.remote_loader.get_remote_usd_cad_rates(year)?;
        for nfe in res.non_fatal_errors {
            let _ = writeln!(self.err_stream, "{}", nfe);
        }
        let rates = fill_in_unknown_day_rates(&res.rates, year);

        self.fresh_loaded_years.insert(year);
        if let Err(e) =  self.cache.write_rates(year, &rates) {
            let _ = writeln!(self.err_stream,
                "Failed to update exchange rate cache: {}", e);
        }

        Ok(rates)
    }

    // TL;DR official recommendation appears to be to get the "active" rate on the trade
    // day, which is the last known rate (we can'tradeDate see the future, obviously).
    //
    // As per the CRA's interpretation of Section 261 (1.4)
    // https://www.canada.ca/en/revenue-agency/services/tax/technical-information/income-tax/income-tax-folios-index/series-5-international-residency/series-5-international-residency-folio-4-foreign-currency/income-tax-folio-s5-f4-c1-income-tax-reporting-currency.html
    //
    // For a particular day after February 28, 2017, the relevant spot rate is to be used to
    // convert an amount from one currency to another, where one of the currencies is
    // Canadian currency is the rate quoted by the Bank of Canada on that day. If the Bank
    // of Canada ordinarily quotes such a rate, but no rate is quoted for that particular
    // day, then the closest preceding day for which such a rate is quoted should be used.
    // If the particular day, or closest preceding day, of conversion is before
    // March 1, 2017, the Bank of Canada noon rate should be used.

    // NOTE: This function should NOT be called for today if the rate is not yet knowable.
    fn find_usd_cad_preceding_relevant_spot_rate(&mut self, trade_date: Date) -> Result<DailyRate, Error> {
        let tax_recommendation = concat!(
            "As per Section 261(1) of the Income Tax Act, the exchange rate ",
		    "from the preceding day for which such a rate is quoted should be ",
		    "used if no rate is quoted on the day the trade.");

        let mut preceding_date = trade_date.clone();
        // Limit to 7 days look-back. This is arbitrarily chosen as a large-enough value
	    // (unless the markets close for more than a week due to an apocalypse)
        for _ in 0..7 {
            preceding_date = preceding_date.saturating_sub(Duration::days(1));
            let rate_opt = match self.get_exact_usd_cad_rate(trade_date) {
                Ok(r) => r,
                Err(e) => {
                    return Err(format!(
                        "Cound not retrieve exchange rates within the 7 preceding days ({}). {}",
                        e, tax_recommendation));
                }
            };
            if let Some(rate) = rate_opt {
                return Ok(rate)
            }
        }

        Err(format!(
            "Could not find relevant exchange rate within the 7 preceding days. {}",
            tax_recommendation))
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::{prelude::Zero, Decimal};
    use rust_decimal_macros::dec;

    use crate::fx::DailyRate;

    use super::fill_in_unknown_day_rates;
    use crate::testlib::assert_vec_eq;
    use crate::util::date::testlib::doy_date;


    #[test]
    fn test_fill_in_unknown_day_rates() {
        let date_yd = |year, doy| { doy_date(year, doy) };
        let dr = |date, rate| { DailyRate{date: date, foreign_to_local_rate: rate} };

        let rates = vec![
            dr(date_yd(2022, 0), dec!(1.0)),
            dr(date_yd(2022, 1), dec!(1.1)),
            dr(date_yd(2022, 2), dec!(1.2)),
        ];

        // Simple no fills
        crate::util::date::set_todays_date_for_test(date_yd(2022, 2));
        assert_vec_eq(
            fill_in_unknown_day_rates(&rates, 2022),
            vec![
                dr(date_yd(2022, 0), dec!(1.0)),
                dr(date_yd(2022, 1), dec!(1.1)),
                dr(date_yd(2022, 2), dec!(1.2)),
            ]
        );

        crate::util::date::set_todays_date_for_test(date_yd(2022, 3));
        assert_vec_eq(
            fill_in_unknown_day_rates(&rates, 2022),
            vec![
                dr(date_yd(2022, 0), dec!(1.0)),
                dr(date_yd(2022, 1), dec!(1.1)),
                dr(date_yd(2022, 2), dec!(1.2)),
            ]
        );

        // End fill only.
        crate::util::date::set_todays_date_for_test(date_yd(2022, 4));
        assert_vec_eq(
            fill_in_unknown_day_rates(&rates, 2022),
            vec![
                dr(date_yd(2022, 0), dec!(1.0)),
                dr(date_yd(2022, 1), dec!(1.1)),
                dr(date_yd(2022, 2), dec!(1.2)),
                dr(date_yd(2022, 3), Decimal::zero()),
            ]
        );

        // Different year
        crate::util::date::set_todays_date_for_test(date_yd(2023, 4));
        assert_eq!(
            fill_in_unknown_day_rates(&rates, 2022).len(),
            365,
        );

        // Middle and front fills
        let rates = vec![
            // dr(date_yd(2022, 0), 1.0),
            dr(date_yd(2022, 1), dec!(1.1)),
            // dr(date_yd(2022, 2), 1.2),
            dr(date_yd(2022, 3), dec!(1.3)),
            dr(date_yd(2022, 4), dec!(1.4)),
            // dr(date_yd(2022, 5), 1.2),
            // dr(date_yd(2022, 6), 1.2),
            dr(date_yd(2022, 7), dec!(1.7)),
        ];

        crate::util::date::set_todays_date_for_test(date_yd(2022, 7));
        assert_vec_eq(
            fill_in_unknown_day_rates(&rates, 2022),
            vec![
                dr(date_yd(2022, 0), Decimal::zero()),
                dr(date_yd(2022, 1), dec!(1.1)),
                dr(date_yd(2022, 2), Decimal::zero()),
                dr(date_yd(2022, 3), dec!(1.3)),
                dr(date_yd(2022, 4), dec!(1.4)),
                dr(date_yd(2022, 5), Decimal::zero()),
                dr(date_yd(2022, 6), Decimal::zero()),
                dr(date_yd(2022, 7), dec!(1.7)),
            ]
        );
    }

    // TODO TestGetEffectiveUsdCadRateFreshCache

    // TODO TestGetEffectiveUsdCadRateWithCache

    // TODO TestGetEffectiveUsdCadRateCacheInvalidation

    // TODO TestGetEffectiveUsdCadRateWithCsvCache
    //      (integration test. not here. create tests/fx_rateloader_test.rs)
}