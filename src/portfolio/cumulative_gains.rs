use std::collections::HashMap;

use rust_decimal::Decimal;

use super::{Security, TxDelta};

pub struct CumulativeCapitalGains {
    // None for registered affiliates (? maybe not).
    pub capital_gains_total: Decimal,
    pub capital_gains_years_totals: HashMap<i32, Decimal>,
}

impl CumulativeCapitalGains {
    pub fn capital_gains_year_totals_keys_sorted(&self) -> Vec<i32> {
        let mut years: Vec<i32> = self.capital_gains_years_totals.keys().copied().collect();
        years.sort();
        years
    }
}

pub fn calc_security_cumulative_capital_gains(deltas: &Vec<TxDelta>) -> CumulativeCapitalGains {
    let mut capital_gains_total = Decimal::ZERO;
    let mut cap_gains_year_totals = HashMap::<i32, Decimal>::new();

    for d in deltas {
        if let Some(cap_gain) = d.capital_gain {
            capital_gains_total += cap_gain;
            let year = d.tx.settlement_date.year();
            let year_total_so_far = cap_gains_year_totals.get(&year).unwrap_or(&Decimal::ZERO);
            cap_gains_year_totals.insert(year, year_total_so_far + cap_gain);
        }
    }

    CumulativeCapitalGains {
        capital_gains_total,
        capital_gains_years_totals: cap_gains_year_totals,
    }
}

pub fn calc_cumulative_capital_gains(sec_gains: &HashMap<Security, CumulativeCapitalGains>) -> CumulativeCapitalGains {
    let mut capital_gains_total = Decimal::ZERO;
    let mut cap_gains_year_totals = HashMap::<i32, Decimal>::new();

    for gains in sec_gains.values() {
        capital_gains_total += gains.capital_gains_total;
        for (year, year_gains) in &gains.capital_gains_years_totals {
            let year_total_so_far = cap_gains_year_totals.get(&year).unwrap_or(&Decimal::ZERO);
            cap_gains_year_totals.insert(*year, year_total_so_far + year_gains);
        }
    }

    CumulativeCapitalGains {
        capital_gains_total,
        capital_gains_years_totals: cap_gains_year_totals,
    }
}
