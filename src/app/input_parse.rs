use std::{collections::HashMap, str::FromStr};

use rust_decimal::Decimal;

use crate::{portfolio::{PortfolioSecurityStatus, Security}, util::decimal::GreaterEqualZeroDecimal};

pub type Error = String;

/// Takes a list of security status strings, each formatted as:
/// SYM:nShares:totalAcb. Eg. GOOG:20:1000.00
pub fn parse_initial_status(
    initial_security_states: &Vec<String>
    ) -> Result<HashMap<Security, PortfolioSecurityStatus>, Error> {

    let mut stati = HashMap::<String, PortfolioSecurityStatus>::with_capacity(
        initial_security_states.len());
    for opt in initial_security_states {
        let mut parts: Vec<String> = opt.split(":").map(|s| s.to_string()).collect();
        if parts.len() != 3 {
            return Err(format!("Invalid ACB format '{opt}'"));
        }
        let acb_str = parts.pop().unwrap();
        let shares_str = parts.pop().unwrap();
        let symbol = parts.pop().unwrap().trim().to_string();
        if symbol.is_empty() {
            return Err("Symbol was empty".to_string());
        }

        let shares = Decimal::from_str(&shares_str)
            .map_err(|e| format!("Invalid shares format '{shares_str}'. {e}"))?;
        let shares = GreaterEqualZeroDecimal::try_from(shares)
            .map_err(|_| format!("Shares {shares} was negative"))?;

        let acb = Decimal::from_str(&acb_str)
            .map_err(|e| format!("Invalid ACB format '{acb_str}'. {e}"))?;
        let acb = GreaterEqualZeroDecimal::try_from(acb)
            .map_err(|_| format!("ACB {acb} was negative"))?;

        stati.insert(symbol.clone(), PortfolioSecurityStatus{
            security: symbol.clone(),
            share_balance: shares,
            all_affiliate_share_balance: shares,
            total_acb: Some(acb),
        });
    }

    Ok(stati)
}

#[cfg(test)]
mod tests {
    use crate::{gezdec, portfolio::PortfolioSecurityStatus};

    use super::parse_initial_status;

    #[test]
    fn test_parse_initial_status() {
        let res = parse_initial_status(&vec![
            "FOO:20:1000.0".to_string(),
            "BAR:0:0".to_string(),
        ]).unwrap();
        assert_eq!(res.len(), 2);
        let status = &res["FOO"];
        assert_eq!(*status, PortfolioSecurityStatus{
            security: "FOO".to_string(),
            share_balance: gezdec!(20),
            all_affiliate_share_balance: gezdec!(20),
            total_acb: Some(gezdec!(1000)),
        });

        // Errors
        let _ = parse_initial_status(&vec![
            "FOO:20:".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            "FOO:20".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            ":20:1234".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            "".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            "FOO:asd:100".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            "FOO:20:sdf".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            "FOO:20:-19".to_string(),
        ]).unwrap_err();
        let _ = parse_initial_status(&vec![
            "FOO:-20:10".to_string(),
        ]).unwrap_err();
    }
}