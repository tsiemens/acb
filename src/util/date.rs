use std::sync::Mutex;

use chrono::Datelike;
use time::{macros::format_description, Month};
pub use time::Date;

use lazy_static::lazy_static;

pub type StaticDateFormat<'a> = &'static [time::format_description::BorrowedFormatItem<'a>];

pub const STANDARD_DATE_FORMAT: StaticDateFormat = format_description!("[year]-[month]-[day]");

pub fn parse_standard_date(date_str: &str) -> Result<Date, time::error::Parse> {
    Date::parse(date_str, STANDARD_DATE_FORMAT)
}

fn date_naive_to_date(dn: &chrono::NaiveDate) -> Date {
    Date::from_calendar_date(
        dn.year(),
        Month::December.nth_next(dn.month() as u8),
        dn.day() as u8)
    .unwrap()
}

lazy_static! {
    static ref TODAYS_DATE_FOR_TEST: Mutex<Date> = Mutex::new(Date::MIN);
}

pub fn set_todays_date_for_test(d: Date) {
    let mut test_date = TODAYS_DATE_FOR_TEST.lock().unwrap();
    *test_date = d;
}

pub fn today_local() -> Date {
    let test_date = TODAYS_DATE_FOR_TEST.lock().unwrap().clone();
    if test_date != Date::MIN {
        return test_date;
    }
    let now = chrono::offset::Local::now();
    date_naive_to_date(&now.date_naive())
}

#[cfg(test)]
pub mod testlib {
    use time::{Date, Duration, Month};

    pub fn doy_date(year: u32, day: i64) -> Date {
        Date::from_calendar_date(year as i32, Month::January, 1).unwrap()
            .saturating_add(Duration::days(day))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use time::{Date, Month};

    use super::{date_naive_to_date, parse_standard_date};

    #[test]
    fn test_parse() {
        let d = parse_standard_date("2023-01-21");
        assert_eq!(d.unwrap(),
            Date::from_calendar_date(2023, Month::January, 21).unwrap());

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
        assert_eq!(date, Date::from_calendar_date(2024, Month::April, 13).unwrap());
    }
}