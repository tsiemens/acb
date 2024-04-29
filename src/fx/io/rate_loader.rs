use std::collections::HashMap;

use rust_decimal::{prelude::Zero, Decimal};
use time::{Date, Duration, Month};

use crate::{fx::DailyRate, util::date::today_local};

use crate::fx::io::RemoteRateLoader;

// Overall utility for loading rates (both remotely and from cache).
pub struct RateLoader {
    pub year_rates: HashMap<u32, HashMap<Date, DailyRate>>,
    pub force_download: bool,
    // pub cache: RatesCache,
    // fresh_loaded_years: HashSet<u32>,
    // err_stream: WriteHandle,
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
}