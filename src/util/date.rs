use std::cell::RefCell;

use chrono::Datelike;
pub use time::Date;
use time::{macros::format_description, Month, UtcOffset};

pub type StaticDateFormat<'a> =
    &'static [time::format_description::BorrowedFormatItem<'a>];
pub type DynDateFormat = time::format_description::OwnedFormatItem;

pub const STANDARD_DATE_FORMAT: StaticDateFormat =
    format_description!("[year]-[month]-[day]");

pub fn parse_standard_date(date_str: &str) -> Result<Date, time::error::Parse> {
    Date::parse(date_str, STANDARD_DATE_FORMAT)
}

pub fn parse_dyn_date_format(fmt: &str) -> Result<DynDateFormat, String> {
    // The documentation recommends version 2
    const VERSION: usize = 2;
    time::format_description::parse_owned::<VERSION>(fmt)
        .map_err(|e| format!("{}", e))
}

pub fn parse_date(
    date_str: &str,
    fmt: &Option<DynDateFormat>,
) -> Result<Date, time::error::Parse> {
    match fmt {
        Some(fmt_) => Date::parse(date_str, &fmt_),
        None => parse_standard_date(date_str),
    }
}

fn date_naive_to_date(dn: &chrono::NaiveDate) -> Date {
    Date::from_calendar_date(
        dn.year(),
        Month::December.nth_next(dn.month() as u8),
        dn.day() as u8,
    )
    .unwrap()
}

pub fn parse_month(m: &str) -> Result<Month, ()> {
    let m_lower = m.to_lowercase();
    let trimmed = m_lower.trim();
    if trimmed.starts_with("jan") {
        Ok(Month::January)
    } else if trimmed.starts_with("feb") {
        Ok(Month::February)
    } else if trimmed.starts_with("mar") {
        Ok(Month::March)
    } else if trimmed.starts_with("apr") {
        Ok(Month::April)
    } else if trimmed.starts_with("may") {
        Ok(Month::May)
    } else if trimmed.starts_with("jun") {
        Ok(Month::June)
    } else if trimmed.starts_with("jul") {
        Ok(Month::July)
    } else if trimmed.starts_with("aug") {
        Ok(Month::August)
    } else if trimmed.starts_with("sep") {
        Ok(Month::September)
    } else if trimmed.starts_with("oct") {
        Ok(Month::October)
    } else if trimmed.starts_with("nov") {
        Ok(Month::November)
    } else if trimmed.starts_with("dec") {
        Ok(Month::December)
    } else {
        Err(())
    }
}

pub fn to_pretty_string(d: &Date) -> String {
    format!("{} {}, {}", d.month(), d.day(), d.year())
}

thread_local! {
    static TODAYS_DATE_FOR_TEST_TL: RefCell<Date> = RefCell::new(Date::MIN);
}

pub fn set_todays_date_for_test(d: Date) {
    TODAYS_DATE_FOR_TEST_TL.with_borrow_mut(|d_| *d_ = d);
}

pub fn today_local() -> Date {
    let test_date: Date = TODAYS_DATE_FOR_TEST_TL.with_borrow(|d| d.clone());
    if test_date != Date::MIN {
        return test_date.clone();
    }
    let now = chrono::offset::Local::now();
    date_naive_to_date(&now.date_naive())
}

// This is a (possibly unsafe, but no worse than today_local) way
// to get the current system UtcOffset of local timezone.
// Using UtcOffset::current_local_offset is apparently unsafe on Linux,
// and will return an error if used without enabling some "unsafe" feature.
// I read that Local::now may be similarly unsafe, but apparently isn't
// blocking itself explicitly, so I guess I'll use it for now. ¯\_(ツ)_/¯
pub fn local_utc_offset() -> Result<UtcOffset, time::error::ComponentRange> {
    let now = chrono::offset::Local::now();
    let offset = now.offset();
    UtcOffset::from_whole_seconds(-1 * offset.utc_minus_local())
}

// Used by both unit and integration tests
pub mod pub_testlib {
    use time::{Date, Duration, Month};

    pub fn doy_date(year: u32, day: i64) -> Date {
        Date::from_calendar_date(year as i32, Month::January, 1)
            .unwrap()
            .saturating_add(Duration::days(day))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use time::{Date, Month};

    use crate::util::date::to_pretty_string;

    use super::{date_naive_to_date, parse_standard_date};

    #[test]
    fn test_parse() {
        let d = parse_standard_date("2023-01-21");
        assert_eq!(
            d.unwrap(),
            Date::from_calendar_date(2023, Month::January, 21).unwrap()
        );

        let d = parse_standard_date("2023-01-41");
        assert!(d.is_err());
    }

    #[test]
    fn test_render() {
        let d = parse_standard_date("2024-01-23");
        assert_eq!(d.unwrap().to_string(), "2024-01-23");
    }

    #[test]
    fn test_date_naive_to_date() {
        let naive_date = NaiveDate::from_ymd_opt(2024, 4, 13).unwrap();
        let date = date_naive_to_date(&naive_date);
        assert_eq!(
            date,
            Date::from_calendar_date(2024, Month::April, 13).unwrap()
        );
    }

    #[test]
    fn test_to_pretty_string() {
        assert_eq!(
            to_pretty_string(&parse_standard_date("2024-03-01").unwrap()),
            "March 1, 2024"
        );
    }
}
