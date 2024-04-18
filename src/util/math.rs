use std::fmt::Display;

use rust_decimal::Decimal;

#[derive(PartialEq, Eq, Debug)]
pub struct DecimalRatio {
    pub numerator: Decimal,
    pub denominator: Decimal,

}

impl DecimalRatio {
    pub fn is_valid(&self) -> bool {
        !self.denominator.is_zero()
    }

    pub fn to_decimal(&self) -> Decimal {
        self.numerator / self.denominator
    }
}

// Auto-implements to_string()
impl Display for DecimalRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}/{}}}", self.numerator, self.denominator)
    }
}