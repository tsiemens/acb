use std::fmt::Display;

use rust_decimal::Decimal;

use super::decimal::{
    constraint, ConstrainedDecimal, DecConstraint, GreaterEqualZeroDecimal,
    PosDecimal,
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ConstrainedDecimalRatio<CONSTRAINT: DecConstraint + Clone + Copy> {
    pub numerator: ConstrainedDecimal<CONSTRAINT>,
    pub denominator: PosDecimal,
}

impl<CONSTRAINT: DecConstraint + Clone + Copy> ConstrainedDecimalRatio<CONSTRAINT> {
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
        GreaterEqualZeroDecimal::try_from(*self.numerator / *self.denominator)
            .unwrap()
    }
}

// Auto-implements to_string()
impl<CONSTRAINT: DecConstraint + Clone + Copy> Display
    for ConstrainedDecimalRatio<CONSTRAINT>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.numerator.fmt(f)?;
        write!(f, "/")?;
        self.denominator.fmt(f)
    }
}

impl<CONSTRAINT: DecConstraint + Clone + Copy> core::fmt::Debug
    for ConstrainedDecimalRatio<CONSTRAINT>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}/{}}}", self.numerator, self.denominator)
    }
}

pub type PosDecimalRatio = ConstrainedDecimalRatio<constraint::Pos>;
pub type GezDecimalRatio = ConstrainedDecimalRatio<constraint::GreaterEqualZero>;

pub fn round_to_cent(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
}

pub fn c_round_to_cent<T: DecConstraint>(
    d: ConstrainedDecimal<T>,
) -> ConstrainedDecimal<T> {
    ConstrainedDecimal::<T>::try_from(round_to_cent(*d)).unwrap()
}

fn round_to_effective_cent_tolerance() -> Decimal {
    // 0.0000000001
    Decimal::new(1, 10)
}

/// Performs some practical rounding, which avoids annoying and unnecessary precision
/// in cases where a currency Decimal value is essentially equal to an exact cent.
///
/// eg. We do not want something like 1.009999999999999999999. We want it to be 1.01.
/// It also behaves the same for all practical purposes.
/// This is not just a float issue, as this can happen with Decimals when multiplied or
/// divided into a real number or fraction that is not decimal-representable
/// (eg. 1.55555555555555556), and then propagated to other values.
pub fn maybe_round_to_effective_cent(d: Decimal) -> Decimal {
    let rounded = round_to_cent(d);
    let tolerance: Decimal = round_to_effective_cent_tolerance();
    if (rounded - d).abs() < tolerance {
        rounded
    } else {
        d
    }
}

pub fn c_maybe_round_to_effective_cent<T: DecConstraint>(
    d: ConstrainedDecimal<T>,
) -> ConstrainedDecimal<T> {
    ConstrainedDecimal::<T>::try_from(maybe_round_to_effective_cent(*d)).unwrap()
}

// MARK: tests
#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::{
        pdec,
        util::math::{maybe_round_to_effective_cent, round_to_cent},
    };

    use super::PosDecimalRatio;

    #[test]
    fn test_ratio_display() {
        let ratio = PosDecimalRatio {
            numerator: pdec!(1.123456),
            denominator: pdec!(9.87654),
        };
        assert_eq!(format!("{ratio}"), "1.123456/9.87654");
        assert_eq!(format!("{ratio:.2}"), "1.12/9.87");
    }

    #[test]
    fn test_round_to_cent() {
        assert_eq!(round_to_cent(dec!(1.99999999)), dec!(2.00));
        assert_eq!(round_to_cent(dec!(1.495)), dec!(1.50));
        assert_eq!(round_to_cent(dec!(1.49499999)), dec!(1.49));
        assert_eq!(round_to_cent(dec!(-1.49499999)), dec!(-1.49));

        // No changes expected
        assert_eq!(round_to_cent(dec!(1.5)), dec!(1.5));
        assert_eq!(round_to_cent(dec!(1.59)), dec!(1.59));
    }

    #[test]
    fn test_maybe_round_to_effective_cent() {
        // Sanity check that our raw-input value (for speed) is what we think it is.
        assert_eq!(
            dec!(0.0000000001),
            super::round_to_effective_cent_tolerance()
        );

        assert_eq!(
            maybe_round_to_effective_cent(dec!(1.99999999999)),
            dec!(2.00)
        );
        assert_eq!(
            maybe_round_to_effective_cent(dec!(1.90999999999999999)),
            dec!(1.91)
        );
        assert_eq!(
            maybe_round_to_effective_cent(dec!(1.320000000000001)),
            dec!(1.32)
        );
        assert_eq!(
            maybe_round_to_effective_cent(dec!(-1.320000000000001)),
            dec!(-1.32)
        );

        // No changes expected
        assert_eq!(
            maybe_round_to_effective_cent(dec!(1.909999)),
            dec!(1.909999)
        );
        assert_eq!(
            maybe_round_to_effective_cent(dec!(1.194444444444444444444)),
            dec!(1.194444444444444444444)
        );
        assert_eq!(
            maybe_round_to_effective_cent(dec!(-1.32001)),
            dec!(-1.32001)
        );
        assert_eq!(maybe_round_to_effective_cent(dec!(1.001)), dec!(1.001));
    }
}
