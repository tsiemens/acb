use std::collections::{HashMap, HashSet};
use std::io::Write;

use rust_decimal::{prelude::Zero, Decimal};
use time::{Date, Duration, Month};
use tracing::{debug, error, info, trace};

use crate::write_errln;
use crate::log::WriteHandle;
use crate::{fx::DailyRate, util::date::today_local};

use crate::fx::io::RemoteRateLoader;

use super::{Error, RatesCache};

// Overall utility for loading rates (both remotely and from cache).
pub struct RateLoader {
    pub force_download: bool,
    pub cache: Box<dyn RatesCache>,
    pub remote_loader: Box<dyn RemoteRateLoader>,
    err_stream: WriteHandle,

    year_rates: HashMap<u32, HashMap<Date, DailyRate>>,
    fresh_loaded_years: HashSet<u32>,
}

impl RateLoader {
    pub fn new(force_download: bool,
               cache: Box<dyn RatesCache>,
               remote_loader: Box<dyn RemoteRateLoader>,
               err_stream: WriteHandle) -> RateLoader {
        RateLoader{
            force_download,
            cache,
            remote_loader,
            year_rates: HashMap::new(),
            fresh_loaded_years: HashSet::new(),
            err_stream: err_stream,
        }
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
            debug!("RateLoader::get_exact_usd_cad_rate {} not yet loaded", year);
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
                write_errln!(self.err_stream, "Could not load cached exchange rates: {}", e);
            } else {
                match cache_res.unwrap() {
                    Some(rates) => {
                        info!("RateLoader::fetch rates found in cache");
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
                        info!("RateLoader::fetch NO rates found in cache");
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
        trace!(year = year, "RateLoader::get_remote_usd_cad_rates");
        let res = self.remote_loader.get_remote_usd_cad_rates(year)?;
        for nfe in res.non_fatal_errors {
            write_errln!(self.err_stream, "{}", nfe);
        }
        let rates = fill_in_unknown_day_rates(&res.rates, year);

        self.fresh_loaded_years.insert(year);
        if let Err(e) =  self.cache.write_rates(year, &rates) {
            error!("RateLoader::get_remote_usd_cad_rates cache write failed: {}", e);
            write_errln!(self.err_stream,
                "Failed to update exchange rate cache: {}", e);
            let _ = self.err_stream.flush();
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
            let rate_opt = match self.get_exact_usd_cad_rate(preceding_date) {
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
    use std::collections::HashMap;

    use rust_decimal::{prelude::Zero, Decimal};
    use rust_decimal_macros::dec;
    use time::Date;

    use crate::fx::io::pub_testlib::MockRemoteRateLoader;
    use crate::fx::io::InMemoryRatesCache;
    use crate::fx::DailyRate;
    use crate::log::WriteHandle;
    use crate::util::rc::{RcRefCell, RcRefCellT};

    use super::{fill_in_unknown_day_rates, RateLoader};
    use crate::testlib::{assert_vec_eq, assert_vecr_eq};
    use crate::util::date::pub_testlib::doy_date;

    fn date_yd(year: u32, doy: i64) -> Date {
        doy_date(year, doy)
    }

    fn dr(date: Date, rate: Decimal) -> DailyRate {
        DailyRate{date: date, foreign_to_local_rate: rate}
    }

    #[test]
    fn test_fill_in_unknown_day_rates() {
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

    fn new_test_rate_loader_with_remote(
        force_download: bool,remote_year_rates: &RcRefCell<HashMap<u32, Vec<DailyRate>>>) ->
        (RateLoader, RcRefCell<HashMap<u32, Vec<DailyRate>>>) {

        let cache_year_rates = RcRefCellT::new(HashMap::new());
        let rate_loader = RateLoader::new(
            force_download,
            Box::new(InMemoryRatesCache{ rates_by_year: cache_year_rates.clone() }),
            Box::new(MockRemoteRateLoader{ remote_year_rates: remote_year_rates.clone() }),
            WriteHandle::empty_write_handle());

        (rate_loader, cache_year_rates)
    }

    fn new_test_rate_loader(force_download: bool) ->
        (RateLoader, RcRefCell<HashMap<u32, Vec<DailyRate>>>, RcRefCell<HashMap<u32, Vec<DailyRate>>>) {
        let cache_year_rates = RcRefCellT::new(HashMap::new());
        let remote_year_rates =  RcRefCellT::new(HashMap::new());
        let rate_loader = RateLoader::new(
            force_download,
            Box::new(InMemoryRatesCache{ rates_by_year: cache_year_rates.clone() }),
            Box::new(MockRemoteRateLoader{ remote_year_rates: remote_year_rates.clone() }),
            WriteHandle::empty_write_handle());

        (rate_loader, cache_year_rates, remote_year_rates)
    }

    #[test]
    fn test_get_effective_usd_cad_rate_fresh_cache() {
        crate::util::date::set_todays_date_for_test(date_yd(2022, 12));

        let (mut rate_loader, cached_rates, remote_rates) =
            new_test_rate_loader(false);
        cached_rates.borrow_mut().insert(2022, vec![]);
        remote_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(1.0)),
            // dr(date_yd(2022, 1), 1.1), // Expect fill here
            dr(date_yd(2022, 2), dec!(1.2)),
            // Expect 9 days fill here (unrealistic)
            dr(date_yd(2022, 12), dec!(2.2)),
        ]);

        // Test failure to get from remote.
        let _ = rate_loader.get_effective_usd_cad_rate(date_yd(1970, 1)).unwrap_err();

        // Test find exact rate
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 0)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 0), dec!(1.0)));
        // Test fall back to previous day rate
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 1)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 0), dec!(1.0)));
        // Test exact match after a fill day
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 2)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 2), dec!(1.2)));
        // Test fall back with the day ahead also not being present
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 7)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 2), dec!(1.2)));
        // Test fall back 7 days (the max allowed)
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 9)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 2), dec!(1.2)));
        // Test fall back 8 days (more than allowed)
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let _ = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 10)).unwrap_err();

        // Test lookup for today's determined (markets opened) rate
        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 12)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 12), dec!(2.2)));

        // Test lookup for today's (undetermined) rate
        crate::util::date::set_todays_date_for_test(date_yd(2022, 13));

        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let _ = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 13)).unwrap_err();

        // Test lookup for yesterday's determined (markets closed) rate
        crate::util::date::set_todays_date_for_test(date_yd(2022, 14));

        let (mut rate_loader, _) = new_test_rate_loader_with_remote(false, &remote_rates);
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 13)).unwrap();
        assert_eq!(geucr_rate, dr(date_yd(2022, 12), dec!(2.2)));
    }

    #[test]
    fn test_get_effective_usd_cad_rate_with_cache() {
        // Sanity check
        assert_eq!(dr(date_yd(2022, 1), dec!(0.0)), dr(date_yd(2022, 1), Decimal::zero()));

        let (mut rate_loader, cached_rates, _) =
            new_test_rate_loader(false);
        cached_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 1), dec!(1.1)),
            dr(date_yd(2022, 2), dec!(0.0)), // Filled (markets closed)
            dr(date_yd(2022, 3), dec!(0.0)), // Filled (markets closed)
        ]);

        // Test lookup of well-known cached value for tomorrow, today, yesterday
        for i in 0..=2 {
            crate::util::date::set_todays_date_for_test(date_yd(2022, i));
            let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 1)).unwrap();
            assert_eq!(geucr_rate, dr(date_yd(2022, 1), dec!(1.1)));
        }
        // Test lookup of defined markets closed cached value for tomorrow, today, yesterday,
        // where later values are present.
        for i in 1..=3 {
            crate::util::date::set_todays_date_for_test(date_yd(2022, i));
            let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 2)).unwrap();
            assert_eq!(geucr_rate, dr(date_yd(2022, 1), dec!(1.1)));
        }
        // Test lookup of defined markets closed cached value for tomorrow, today, yesterday,
        // where this is the last cached value.
        for i in 2..=4 {
            crate::util::date::set_todays_date_for_test(date_yd(2022, i));
            let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 3)).unwrap();
            assert_eq!(geucr_rate, dr(date_yd(2022, 1), dec!(1.1)));
        }
    }

    #[test]
    fn test_get_effective_usd_cad_rate_cache_invalidation() {
        // Test cache invalidates when querying today with no cached value, and there is
        // no remote value
        let (mut rate_loader, cached_rates, remote_rates) =
            new_test_rate_loader(false);
        // rateLoader, ratesCache, remote := NewTestRateLoader(false)
        cached_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 1), dec!(1.1)),
        ]);
        remote_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 1), dec!(1.4)), // Value change.
        ]);
        crate::util::date::set_todays_date_for_test(date_yd(2022, 2));
        // Can't use today unless it's been published or specified.
        let _ = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 2)).unwrap_err();
        assert_vecr_eq(
            cached_rates.borrow().get(&2022).unwrap(),
            &vec![
                dr(date_yd(2022, 0), dec!(0.0)), // fill
                dr(date_yd(2022, 1), dec!(1.4)),
            ]);

        // Test cache invalidates when querying today with no cached value, and there is
        // a remote value
        let (mut rate_loader, cached_rates, remote_rates) =
            new_test_rate_loader(false);
        // rateLoader, ratesCache, remote = NewTestRateLoader(false)
        cached_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(1.0)),
        ]);
        remote_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(1.0)),
            dr(date_yd(2022, 1), dec!(1.1)),
        ]);
        crate::util::date::set_todays_date_for_test(date_yd(2022, 1));
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 1)).unwrap();
        assert_vecr_eq(cached_rates.borrow().get(&2022).unwrap(),
                       remote_rates.borrow().get(&2022).unwrap());
        assert_eq!(geucr_rate, dr(date_yd(2022, 1), dec!(1.1)));

        // Test cache invalidates when querying a previous day with no cached value.
        let (mut rate_loader, cached_rates, remote_rates) =
            new_test_rate_loader(false);
        // rateLoader, ratesCache, remote = NewTestRateLoader(false)
        cached_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(1.0)),
        ]);
        remote_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(1.0)),
            dr(date_yd(2022, 1), dec!(1.1)),
        ]);
        crate::util::date::set_todays_date_for_test(date_yd(2022, 4));
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 1)).unwrap();
        assert_vecr_eq(
            cached_rates.borrow().get(&2022).unwrap(),
            &vec![
                dr(date_yd(2022, 0), dec!(1.0)),
                dr(date_yd(2022, 1), dec!(1.1)),
                dr(date_yd(2022, 2), dec!(0.0)), // fill to yesterday
                dr(date_yd(2022, 3), dec!(0.0)), // fill to yesterday
            ]);
        assert_eq!(geucr_rate, dr(date_yd(2022, 1), dec!(1.1)));

        // Test cache does not invalidate when querying today with no cached value,
        // after we already invalidated and refreshed the cache with this Loader instance.
        remote_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(99.0)),
            dr(date_yd(2022, 1), dec!(99.1)),
        ]);
        // Can't use today unless it's been published or specified.
        let _ = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 4)).unwrap_err();
        // Cache should be unchanged
        assert_vecr_eq(
            cached_rates.borrow().get(&2022).unwrap(),
            &vec![
                dr(date_yd(2022, 0), dec!(1.0)),
                dr(date_yd(2022, 1), dec!(1.1)),
                dr(date_yd(2022, 2), dec!(0.0)),
                dr(date_yd(2022, 3), dec!(0.0)),
            ]);

        // Test force download
        let (mut rate_loader, cached_rates, remote_rates) =
            new_test_rate_loader(true);
        // rateLoader, ratesCache, remote = NewTestRateLoader(false)
        // rateLoader.ForceDownload = true
        cached_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(1.0)),
        ]);
        remote_rates.borrow_mut().insert(2022, vec![
            dr(date_yd(2022, 0), dec!(99.0)),
        ]);
        crate::util::date::set_todays_date_for_test(date_yd(2022, 1));
        let geucr_rate = rate_loader.get_effective_usd_cad_rate(date_yd(2022, 0)).unwrap();
        assert_vecr_eq(cached_rates.borrow().get(&2022).unwrap(),
                       remote_rates.borrow().get(&2022).unwrap());
        assert_eq!(geucr_rate, dr(date_yd(2022, 0), dec!(99.0)));
    }
}