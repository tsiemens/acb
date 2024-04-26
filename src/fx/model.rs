use std::fmt::Display;

use rust_decimal::Decimal;
use time::Date;

#[derive(PartialEq, Eq, Debug)]
pub struct DailyRate {
    pub date: Date,
    pub foreign_to_local_rate: Decimal,
}

// Auto-implements to_string()
impl Display for DailyRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} : {}", self.date, self.foreign_to_local_rate)
    }
}


#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use time::{Date, Month};

    use super::DailyRate;

    #[test]
    fn test_rate_string() {
        let rate = DailyRate{
            date: Date::from_calendar_date(2024, Month::January, 23).unwrap(),
            foreign_to_local_rate: dec!(1.1),
        };
        assert_eq!(rate.to_string(), "2024-01-23 : 1.1");
    }
}