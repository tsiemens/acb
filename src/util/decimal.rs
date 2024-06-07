use std::{fmt::Display, marker::PhantomData, ops::Deref};

use rust_decimal::Decimal;

pub fn min(ds: &[Decimal]) -> Decimal {
    let mut m = ds[0];
    for d in &ds[1..] {
        if *d < m {
            m = *d;
        }
    }
    m
}

use self::constraint::{GreaterEqualZero, LessEqualZero, Neg, Pos};

// These were deprecated as methods on Decimal, so re-implement them.
// Those implementations don't actually do zero checks, and can result
// in weird behaviour.
pub fn is_positive(d: &Decimal) -> bool {
    d.is_sign_positive() && !d.is_zero()
}

pub fn is_negative(d: &Decimal) -> bool {
    d.is_sign_negative() && !d.is_zero()
}

pub fn dollar_precision_str(d: &Decimal) -> String {
    format!("{:.2}", d)
}

pub trait DecConstraint {
    fn is_ok(d: &Decimal) -> bool;
}

pub mod constraint {
    use rust_decimal::Decimal;

    use super::{is_negative, is_positive, DecConstraint};

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct Neg(());
    impl DecConstraint for Neg {
        fn is_ok(d: &Decimal) -> bool {
            is_negative(d)
        }
    }

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct LessEqualZero(());
    impl DecConstraint for LessEqualZero {
        fn is_ok(d: &Decimal) -> bool {
            d.is_sign_negative() || d.is_zero()
        }
    }

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct GreaterEqualZero(());
    impl DecConstraint for GreaterEqualZero {
        fn is_ok(d: &Decimal) -> bool {
            d.is_sign_positive() || d.is_zero()
        }
    }

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct Pos(());
    impl DecConstraint for Pos {
        fn is_ok(d: &Decimal) -> bool {
            is_positive(d)
        }
    }
}

// A constrained instance of Decimal. This can only be created through ::try_from,
// which will enforce the DecConstraint. This allows for a convenient and type-safe
// way to enforce what values any given value can contain.
//
// PhantomData here is size zero, and is simply to make the compiler happy.
// Otherwise, it will complain that the generic parameter is unused (even though
// we are using it in the impl).
pub struct ConstrainedDecimal<CONSTRAINT>(Decimal, PhantomData<CONSTRAINT>);

impl<CONSTRAINT: DecConstraint> TryFrom<Decimal> for ConstrainedDecimal<CONSTRAINT> {
    type Error = String;

    fn try_from(d: Decimal) -> Result<Self, Self::Error> {
        if CONSTRAINT::is_ok(&d) {
            Ok(Self(d, PhantomData))
        } else {
            Err(format!(
                "{} does not match constraints of {}",
                d,
                std::any::type_name::<CONSTRAINT>()
            ))
        }
    }
}

impl<CONSTRAINT: DecConstraint> Deref for ConstrainedDecimal<CONSTRAINT> {
    type Target = Decimal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<CONSTRAINT: DecConstraint> Display for ConstrainedDecimal<CONSTRAINT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<CONSTRAINT: DecConstraint> std::fmt::Debug for ConstrainedDecimal<CONSTRAINT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl<CONSTRAINT: DecConstraint> PartialEq for ConstrainedDecimal<CONSTRAINT> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<CONSTRAINT: DecConstraint> Eq for ConstrainedDecimal<CONSTRAINT> {}

impl<CONSTRAINT: DecConstraint> Clone for ConstrainedDecimal<CONSTRAINT> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<CONSTRAINT: DecConstraint> Copy for ConstrainedDecimal<CONSTRAINT> {}

impl std::ops::Add for ConstrainedDecimal<GreaterEqualZero> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        // GEZ + GEZ will never violate its own constraint
        GreaterEqualZeroDecimal::try_from(*self + *rhs).unwrap()
    }
}

impl std::ops::AddAssign for ConstrainedDecimal<GreaterEqualZero> {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs;
    }
}

impl std::ops::Mul for ConstrainedDecimal<GreaterEqualZero> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        // GEZ * GEZ will never violate its own constraint
        GreaterEqualZeroDecimal::try_from(*self * *rhs).unwrap()
    }
}

impl From<ConstrainedDecimal<constraint::Pos>>
    for ConstrainedDecimal<GreaterEqualZero>
{
    fn from(value: ConstrainedDecimal<constraint::Pos>) -> Self {
        GreaterEqualZeroDecimal::try_from(*value).unwrap()
    }
}

impl ConstrainedDecimal<GreaterEqualZero> {
    pub fn zero() -> Self {
        Self(Decimal::ZERO, PhantomData)
    }

    pub fn div(self, rhs: ConstrainedDecimal<constraint::Pos>) -> Self {
        // GEZ * Pos will never violate its own constraint, or divide by zero
        GreaterEqualZeroDecimal::try_from(*self / *rhs).unwrap()
    }
}

impl std::ops::Mul for ConstrainedDecimal<Pos> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        // Pos * Pos will never violate its own constraint
        PosDecimal::try_from(*self * *rhs).unwrap()
    }
}

impl ConstrainedDecimal<Pos> {
    pub fn one() -> Self {
        PosDecimal::try_from(rust_decimal_macros::dec!(1)).unwrap()
    }
}

impl ConstrainedDecimal<LessEqualZero> {
    pub fn zero() -> Self {
        Self(Decimal::ZERO, PhantomData)
    }
}

impl From<ConstrainedDecimal<constraint::Neg>>
    for ConstrainedDecimal<LessEqualZero>
{
    fn from(value: ConstrainedDecimal<constraint::Neg>) -> Self {
        LessEqualZeroDecimal::try_from(*value).unwrap()
    }
}

impl std::ops::Mul for ConstrainedDecimal<Neg> {
    type Output = PosDecimal;

    fn mul(self, rhs: Self) -> Self::Output {
        // Neg * Neg will never violate its own constraint
        PosDecimal::try_from(*self * *rhs).unwrap()
    }
}

impl std::ops::Div for ConstrainedDecimal<Neg> {
    type Output = PosDecimal;

    fn div(self, rhs: Self) -> Self::Output {
        // Neg / Neg will never violate its own constraint
        PosDecimal::try_from(*self / *rhs).unwrap()
    }
}

impl ConstrainedDecimal<Neg> {
    pub fn neg_1() -> Self {
        NegDecimal::try_from(rust_decimal_macros::dec!(-1)).unwrap()
    }

    pub fn mul_pos(&self, pos_rhs: PosDecimal) -> Self {
        // Neg * Pos will never violate its own constraint
        NegDecimal::try_from(self.0 * *pos_rhs).unwrap()
    }
}

pub fn constrained_min<CONSTRAINT: DecConstraint>(
    ds: &[ConstrainedDecimal<CONSTRAINT>],
) -> ConstrainedDecimal<CONSTRAINT> {
    let mut m = ds[0];
    for d in &ds[1..] {
        if **d < *m {
            m = *d;
        }
    }
    m
}

// Convenience aliases
pub type NegDecimal = ConstrainedDecimal<constraint::Neg>;
pub type LessEqualZeroDecimal = ConstrainedDecimal<constraint::LessEqualZero>;
pub type GreaterEqualZeroDecimal = ConstrainedDecimal<constraint::GreaterEqualZero>;
pub type PosDecimal = ConstrainedDecimal<constraint::Pos>;

#[macro_export]
macro_rules! pdec {
    ($arg:literal) => {{
        use rust_decimal_macros::dec;
        $crate::util::decimal::PosDecimal::try_from(dec!($arg)).unwrap()
    }};
}

#[macro_export]
macro_rules! gezdec {
    ($arg:literal) => {{
        use rust_decimal_macros::dec;
        $crate::util::decimal::GreaterEqualZeroDecimal::try_from(dec!($arg)).unwrap()
    }};
}

#[macro_export]
macro_rules! lezdec {
    ($arg:literal) => {{
        use rust_decimal_macros::dec;
        $crate::util::decimal::LessEqualZeroDecimal::try_from(dec!($arg)).unwrap()
    }};
}

#[macro_export]
macro_rules! ndec {
    ($arg:literal) => {{
        use rust_decimal_macros::dec;
        $crate::util::decimal::NegDecimal::try_from(dec!($arg)).unwrap()
    }};
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::util::decimal::{
        dollar_precision_str, is_negative, is_positive, ConstrainedDecimal,
    };

    use super::{constraint, DecConstraint};

    #[test]
    #[should_panic]
    #[allow(unused)]
    fn test_decimal_div_sanity() {
        // Test that Decimal does not allow NaN, and will panic if we
        // try to do create a decimal with NaN.
        dec!(1) / dec!(0);
    }

    #[test]
    fn test_decimal_sign_sanity() {
        // Check here that this is technically possible.
        // Though doing dec!(-0) apparently won't yield this.
        let mut neg_zero = dec!(0);
        assert!(!neg_zero.is_sign_negative());
        neg_zero.set_sign_negative(true);
        assert_eq!(neg_zero.to_string(), "-0");
        assert!(!is_negative(&neg_zero));
        // This is kind of unexpected, and non-ideal.
        assert!(neg_zero.is_sign_negative());
        assert!(neg_zero.is_zero());
        // Stays sane
        assert!(!is_negative(&neg_zero));

        let mut zero = dec!(0);
        assert!(zero.is_sign_positive());
        assert!(!is_positive(&zero));
        zero.set_sign_positive(true);
        assert!(zero.is_sign_positive());
        assert!(zero.is_zero());
        // Stays sane
        assert!(!is_positive(&zero));

        // This is what really matters though. Zero is always equal.
        assert_eq!(zero, neg_zero);
    }

    #[test]
    fn test_constrained_decimal() {
        _test_constrained_decimal::<constraint::Neg>(
            vec![dec!(-1)],
            vec![dec!(-0), dec!(0), dec!(1)],
        );

        _test_constrained_decimal::<constraint::LessEqualZero>(
            vec![dec!(-1), dec!(0), dec!(-0)],
            vec![dec!(1)],
        );

        _test_constrained_decimal::<constraint::GreaterEqualZero>(
            vec![dec!(1), dec!(0), dec!(-0)],
            vec![dec!(-1)],
        );

        _test_constrained_decimal::<constraint::Pos>(
            vec![dec!(1)],
            vec![dec!(-0), dec!(0), dec!(-1)],
        );
    }

    fn _test_constrained_decimal<C: DecConstraint>(
        dec_vals: Vec<Decimal>,
        invalid_dec_vals: Vec<Decimal>,
    ) {
        for inv in invalid_dec_vals {
            let _ = ConstrainedDecimal::<C>::try_from(inv).unwrap_err();
        }

        for dec_val in dec_vals {
            let valid_val = ConstrainedDecimal::<C>::try_from(dec_val).unwrap();

            assert_eq!(*valid_val, dec_val);
            assert_eq!(valid_val.to_string(), dec_val.to_string());
            assert_eq!(format!("{}", valid_val), format!("{}", dec_val));
            assert_eq!(format!("{:#?}", valid_val), format!("{:#?}", dec_val));
        }
    }

    #[test]
    fn test_dollar_precision_str() {
        assert_eq!(dollar_precision_str(&dec!(1000)), "1000.00");
        assert_eq!(dollar_precision_str(&dec!(1.123456)), "1.12");
    }
}
