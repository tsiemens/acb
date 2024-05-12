use std::fmt::Display;

use rust_decimal_macros::dec;

use crate::{pdec, util::decimal::PosDecimal};

#[derive(Clone, Debug)]
enum CurrImpl {
    Static(&'static str),
    Dyn(String),
}

#[derive(Clone, Debug)]
pub struct Currency(CurrImpl);

impl Currency {
    pub fn new(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "" => Currency::default(),
            "CAD" => Currency::cad(),
            "USD" => Currency::usd(),
            other => Currency(CurrImpl::Dyn(other.to_uppercase())),
        }
    }

    pub fn cad() -> Self {
        Currency(CurrImpl::Static("CAD"))
    }

    pub fn usd() -> Self {
        Currency(CurrImpl::Static("USD"))
    }

    pub fn default() -> Self {
        Currency::cad()
    }

    pub fn is_default(&self) -> bool {
        *self == Currency::default()
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            CurrImpl::Static(s) => s,
            CurrImpl::Dyn(s) => s.as_str(),
        }
    }
}

impl PartialEq for Currency {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Currency {}

// Auto-implements to_string()
impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CurrencyAndExchangeRate {
    pub currency: Currency,
    pub exchange_rate: PosDecimal,
}

impl CurrencyAndExchangeRate {
    pub fn try_new(c: Currency, r: PosDecimal) -> Result<Self, String> {
        if c == Currency::default() && *r != dec!(1.0) {
            return Err(format!("Default currency (CAD) exchange rate was not 1 (was {})",
                               r));
        }
        Ok(Self{currency: c, exchange_rate: r})
    }

    pub fn rq_new(c: Currency, r: PosDecimal) -> Self {
        CurrencyAndExchangeRate::try_new(c, r).unwrap()
    }

    pub fn cad() -> Self {
        Self::rq_new(Currency::cad(), pdec!(1))
    }

    // Just aliases to CAD
    pub fn default() -> Self {
        Self::cad()
    }

    pub fn is_default(&self) -> bool {
        self.currency.is_default()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::{pdec, portfolio::{Currency, CurrencyAndExchangeRate}};

    #[test]
    fn test_currency() {
        assert_eq!(Currency::new("cad"), Currency::new("CAD"));
        assert_eq!(Currency::cad(), Currency::new("CAD"));
        assert_eq!(Currency::cad(), Currency::default());
        assert_eq!(Currency::cad(), Currency::new("Cad"));
        assert_eq!(Currency::new("RMB"), Currency::new("rmb"));

        assert_ne!(Currency::cad(), Currency::usd());
        assert_ne!(Currency::cad(), Currency::new("RMB"));
    }

    #[test]
    fn test_good_currency_rates() {
        let cr = CurrencyAndExchangeRate::rq_new(Currency::usd(), pdec!(1.3));
        assert_eq!(cr.currency, Currency::usd());
        assert_eq!(cr.exchange_rate, pdec!(1.3));

        assert_eq!(CurrencyAndExchangeRate::default(), CurrencyAndExchangeRate::cad());
        assert_eq!(CurrencyAndExchangeRate::cad(), CurrencyAndExchangeRate::rq_new(Currency::cad(), pdec!(1.0)));
    }

    #[test]
    #[should_panic]
    fn test_bad_cad_rate() {
        // Must be 1.0
        CurrencyAndExchangeRate::rq_new(Currency::cad(), pdec!(1.3));
    }

    #[test]
    #[should_panic]
    fn test_zero_rate() {
        // This is actually going to panic because pdec (PosDecimal) itself
        // will fail to unwrap.
        CurrencyAndExchangeRate::rq_new(Currency::usd(), pdec!(0));
    }

    #[test]
    #[should_panic]
    fn test_negative_rate() {
        // This is actually going to panic because pdec (PosDecimal) itself
        // will fail to unwrap.
        CurrencyAndExchangeRate::rq_new(Currency::usd(), pdec!(-1.0));
    }
}