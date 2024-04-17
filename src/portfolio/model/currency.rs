use std::fmt::Display;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

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

#[derive(PartialEq, Eq, Debug)]
pub struct CurrencyAndExchangeRate {
    pub currency: Currency,
    pub exchange_rate: Decimal,
}

impl CurrencyAndExchangeRate {
    pub fn new(c: Currency, r: Decimal) -> Self {
        if c == Currency::default() && r != dec!(1.0) {
            panic!("Default currency (CAD) exchange rate was not 1 (was {})",
                    r);
        }
        if r <= dec!(0) {
            panic!("Exchange rate was not positive (was {})", r);
        }
        Self{currency: c, exchange_rate: r}
    }

    pub fn cad() -> Self {
        Self::new(Currency::cad(), dec!(1))
    }

    pub fn default() -> Self {
        Self::cad()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::portfolio::{Currency, CurrencyAndExchangeRate};

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
        let cr = CurrencyAndExchangeRate::new(Currency::usd(), dec!(1.3));
        assert_eq!(cr.currency, Currency::usd());
        assert_eq!(cr.exchange_rate, dec!(1.3));

        assert_eq!(CurrencyAndExchangeRate::default(), CurrencyAndExchangeRate::cad());
        assert_eq!(CurrencyAndExchangeRate::cad(), CurrencyAndExchangeRate::new(Currency::cad(), dec!(1.0)));
    }

    #[test]
    #[should_panic]
    fn test_bad_cad_rate() {
        // Must be 1.0
        CurrencyAndExchangeRate::new(Currency::cad(), dec!(1.3));
    }

    #[test]
    #[should_panic]
    fn test_zero_rate() {
        CurrencyAndExchangeRate::new(Currency::usd(), dec!(0));
    }

    #[test]
    #[should_panic]
    fn test_negative_rate() {
        CurrencyAndExchangeRate::new(Currency::usd(), dec!(-1.0));
    }
}