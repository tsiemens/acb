use std::collections::{HashMap, HashSet};

use time::Date;

use crate::{portfolio::{Security, TxDelta}, util::decimal::GreaterEqualZeroDecimal};

/// MaxSingleDayCosts helps track the max cost (ACB) of every held/active security
/// on a particular day, as well as the portfolio's total ACB across all
/// securities on that day.
#[derive(Clone)]
pub struct MaxSingleDayCosts {
    // the date (based on settlement date) at which the costs were observed
    pub day: Date,
    // the total ACB of all securities on that date
    pub total: GreaterEqualZeroDecimal,
    // the securities values, which should sum to total
    pub sec_max_cost_for_day: HashMap<Security, GreaterEqualZeroDecimal>,
}

impl MaxSingleDayCosts {
    pub fn new(d: Date) -> Self {
        Self {
            day: d,
            total: GreaterEqualZeroDecimal::zero(),
            sec_max_cost_for_day: HashMap::new(),
        }
    }

    pub fn observe_new_cost(&mut self, sec: &Security, new_cost: GreaterEqualZeroDecimal) {
        let old_day_max_cost = self.sec_max_cost_for_day.get(sec)
            .map(|v| *v)
            .unwrap_or(GreaterEqualZeroDecimal::zero());
        let cur_day_max_cost = GreaterEqualZeroDecimal::try_from(
            old_day_max_cost.max(*new_cost)).unwrap();
            self.sec_max_cost_for_day.insert(sec.clone(), cur_day_max_cost);

        // This should never fail, if everything is sane.
        self.total = GreaterEqualZeroDecimal::try_from(
            *self.total - *old_day_max_cost + *cur_day_max_cost).unwrap();
    }
}

pub struct MaxDayCosts {
    pub max_costs_by_day: HashMap<Date, MaxSingleDayCosts>,
    pub security_set: HashSet<Security>,
    // Translates to notes in final rendered output
    pub ignored_delta_descs: Vec<String>,
}

/// Runs over all_deltas, and generates a MaxSingleDayCosts for every
/// day for which there is a delta (based on the settlement date).
/// Each MaxSingleDayCosts::sec_max_cost_for_day will at least contain
/// an entry for every security held on that day.
///
/// Deltas need not be pre-sorted between securities (interlaced),
/// however, deltas for the same security must be chronologically sorted.
fn calc_max_day_cost_per_sec(all_deltas: &Vec<TxDelta>) -> MaxDayCosts {
    let mut max_costs_by_day = HashMap::<Date, MaxSingleDayCosts>::new();

    // The earliest pre-status cost for all securities
    let mut day_zero_sec_costs = HashMap::<Security, (Date, GreaterEqualZeroDecimal)>::new();

    // For each MomentaryCosts::sec_cost, we need to include every security
    let mut security_set = HashSet::<Security>::new();

    let mut ignored_delta_descs = Vec::<String>::new();

    // Keep track of the maximum cost for each security on any date where there's a TxDelta.
    // For example, SECA on 2000-01-01 has ACB 12, ACB 150, and ACB 0, so after the loop below,
    // we'll have a dateCosts[2001-01-01][SECA] = 150
    for d in all_deltas {
        let date_from_delta = d.tx.settlement_date;
        let sec = &d.post_status.security;

        let total_acb = match d.post_status.total_acb {
            Some(total_acb) => total_acb,
            None => {
                ignored_delta_descs.push(format!(
                    "{date_from_delta} ({sec}) ignored transaction from registered affiliate"));
                continue;
            },
        };
        if !d.tx.affiliate.is_default() {
            let af_name = d.tx.affiliate.name();
            ignored_delta_descs.push(format!(
                "{date_from_delta} ({sec}) ignored transaction from non-default affiliate {af_name}"));
            continue;
        }

        security_set.insert(sec.clone());

        if !max_costs_by_day.contains_key(&date_from_delta) {
            max_costs_by_day.insert(date_from_delta, MaxSingleDayCosts::new(date_from_delta));
        }
        let day_max_costs: &mut MaxSingleDayCosts =
            max_costs_by_day.get_mut(&date_from_delta).unwrap();
        day_max_costs.observe_new_cost(sec, total_acb);

        if !day_zero_sec_costs.contains_key(sec) {
            day_zero_sec_costs.insert(sec.clone(), (date_from_delta, d.pre_status.total_acb.unwrap()));
        } else if day_zero_sec_costs.get(sec).unwrap().0 > date_from_delta {
            panic!("Deltas for {sec} were not sorted by settlement date");
        }
    }

    let mut sorted_days: Vec<Date> = max_costs_by_day.keys().map(|d| *d).collect();
    sorted_days.sort();
    let sorted_days = sorted_days; // finalize

    // Go through each day and populate the ACB for every seen security in each MaxSingleDayCosts
    let mut last_acbs = HashMap::<Security, GreaterEqualZeroDecimal>::new();
    for day in sorted_days {
        let max_costs = max_costs_by_day.get_mut(&day).unwrap();
        for sec in &security_set {
            let last_acb = *max_costs.sec_max_cost_for_day.get(sec)
                .or_else(|| last_acbs.get(sec))
                .unwrap_or_else(|| &day_zero_sec_costs.get(sec).unwrap().1);

            last_acbs.insert(sec.clone(), last_acb);
            if !max_costs.sec_max_cost_for_day.contains_key(sec) {
                max_costs.observe_new_cost(sec, last_acb);
            }
        }
    }

    MaxDayCosts{
        max_costs_by_day,
        security_set,
        ignored_delta_descs,
    }
}

struct YearlyMaxCosts {
    max_costs_for_year: HashMap<i32, MaxSingleDayCosts>,
}

fn calc_yearly_max_cost_day(max_day_costs: &MaxDayCosts) -> YearlyMaxCosts {
    let mut max_cost_day_for_year = HashMap::<i32, Date>::new();

    for (day, day_cost) in &max_day_costs.max_costs_by_day {
        match max_cost_day_for_year.get(&day.year()) {
            Some(old_date) => {
                let old_date_cost = max_day_costs.max_costs_by_day.get(old_date).unwrap();
                if *old_date_cost.total < *day_cost.total {
                    max_cost_day_for_year.insert(day.year(), day.clone());
                }
            },
            None => {
                max_cost_day_for_year.insert(day.year(), day.clone());
            },
        }
    }

    let mut max_costs_for_year = HashMap::<i32, MaxSingleDayCosts>::new();
    for (year, date) in max_cost_day_for_year {
        max_costs_for_year.insert(
            year, max_day_costs.max_costs_by_day.get(&date).unwrap().clone());
    }

    YearlyMaxCosts{ max_costs_for_year }
}

pub struct Costs {
    pub security_set: HashSet<Security>,
    // Complete/total
    pub total: Vec<MaxSingleDayCosts>,
    pub yearly: HashMap<i32, MaxSingleDayCosts>,
    // Translates to notes in rendered output
    pub ignored_deltas: Vec<String>,
}

impl Costs {
    pub fn sorted_years(&self) -> Vec<i32> {
        let mut years: Vec<i32> = self.yearly.keys().map(|y| *y).collect();
        years.sort();
        years
    }
}

/// calc_total_costs generates two sets of MaxSingleDayCosts to determine the maximum
/// total cost of all included securities at any point during the year, and then to find the maximum
/// value of each year.
///
/// We do this by looking at any date with one or more TxDelta settlements. We take the maximum for the
/// day (in the case of multiple settlements on a given day for the same security), and then add that to
/// the current ACB for all the other tracked securities on that day. If there isn't a TxDelta settlement
/// for any of the securities on a given day, we use the last value seen. For this reason we must process
/// all the TxDeltas in date order.
//
// (Non-doc comment, since we don't want the intendation here to be treated
// as doc code, and be compiled).
//
// Examples:
//
//    Total:
//
//             DATE    |  TOTAL  |  SECA   |  XXXX
//        -------------+---------+---------+---------
//          2001-01-13 | $100.00 | $100.00 | $0.00
//        -------------+---------+---------+---------
//          2001-02-14 | $190.00 | $100.00 | $90.00
//        -------------+---------+---------+---------
//          2001-03-15 | $90.00  | $0.00   | $90.00
//        -------------+---------+---------+---------
//          2001-04-16 | $80.00  | $0.00   | $80.00
//        -------------+---------+---------+---------
//          2001-05-17 | $270.00 | $200.00 | $70.00
//        -------------+---------+---------+---------
//          2003-01-01 | $70.00  | $0.00   | $70.00
//        -------------+---------+---------+---------
//        2003-01-02 (TFSA) ignored transaction from registered affiliate
//
//    Yearly:
//
//          YEAR |    DATE    |  TOTAL  |  SECA   |  XXXX
//        -------+------------+---------+---------+---------
//          2001 | 2001-05-17 | $270.00 | $200.00 | $70.00
//        -------+------------+---------+---------+---------
//          2003 | 2003-01-01 | $70.00  | $0.00   | $70.00
//        -------+------------+---------+---------+---------
//        2003-01-02 (TFSA) ignored transaction from registered affiliate
//
pub fn calc_total_costs(all_deltas: &Vec<TxDelta>) -> Costs {
    let mut max_day_costs = calc_max_day_cost_per_sec(all_deltas);

    let max_year_costs = calc_yearly_max_cost_day(&max_day_costs);

    let mut sorted_days: Vec<Date> = max_day_costs.max_costs_by_day.keys().map(|d| *d).collect();
    sorted_days.sort();
    let sorted_days = sorted_days; // finalize

    let mut all_days_max_costs = Vec::<MaxSingleDayCosts>::with_capacity(max_day_costs.max_costs_by_day.len());
    for day in sorted_days {
        all_days_max_costs.push(max_day_costs.max_costs_by_day.remove(&day).unwrap());
    }

    Costs {
        security_set: max_day_costs.security_set,
        total: all_days_max_costs,
        yearly: max_year_costs.max_costs_for_year,
        ignored_deltas: max_day_costs.ignored_delta_descs,
    }
}

// MARK: Tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_calc_max_day_cost_per_sec() {


    }
}