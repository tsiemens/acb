use std::fmt::Display;

use rust_decimal::Decimal;

use super::decimal::{constraint, ConstrainedDecimal, DecConstraint, GreaterEqualZeroDecimal, PosDecimal};

#[derive(PartialEq, Eq, Debug)]
pub struct ConstrainedDecimalRatio<CONSTRAINT: DecConstraint> {
    pub numerator: ConstrainedDecimal<CONSTRAINT>,
    pub denominator: PosDecimal,

}

impl <CONSTRAINT: DecConstraint> ConstrainedDecimalRatio<CONSTRAINT> {
    pub fn to_decimal(&self) -> Decimal {
        *self.numerator / *self.denominator
    }
}

impl ConstrainedDecimalRatio<constraint::Pos> {
    pub fn to_posdecimal(&self) -> PosDecimal {
        PosDecimal::try_from(*self.numerator / *self.denominator).unwrap()
    }
}

impl ConstrainedDecimalRatio<constraint::GreaterEqualZero> {
    pub fn to_gezdecimal(&self) -> GreaterEqualZeroDecimal {
        GreaterEqualZeroDecimal::try_from(*self.numerator / *self.denominator).unwrap()
    }
}

// Auto-implements to_string()
impl <CONSTRAINT: DecConstraint>  Display for ConstrainedDecimalRatio<CONSTRAINT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}/{}}}", self.numerator, self.denominator)
    }
}

pub type PosDecimalRatio = ConstrainedDecimalRatio<constraint::Pos>;
pub type GezDecimalRatio = ConstrainedDecimalRatio<constraint::GreaterEqualZero>;