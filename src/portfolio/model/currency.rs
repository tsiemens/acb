use std::fmt::Display;

#[derive(Debug)]

enum CurrImpl {
    Static(&'static str),
    Dyn(String),
}

#[derive(Debug)]
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

// Auto-implements to_string()
impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::portfolio::Currency;

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
}