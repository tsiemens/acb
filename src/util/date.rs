use time::macros::format_description;
pub use time::Date;

pub type StaticDateFormat<'a> = &'static [time::format_description::BorrowedFormatItem<'a>];

pub const STANDARD_DATE_FORMAT: StaticDateFormat = format_description!("[year]-[month]-[day]");

pub fn parse_standard_date(date_str: &str) -> Result<Date, time::error::Parse> {
    Date::parse(date_str, STANDARD_DATE_FORMAT)
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
    use time::{Date, Month};

    use super::parse_standard_date;

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
}