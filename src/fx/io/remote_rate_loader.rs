use std::{collections::HashSet, str::FromStr};

use json::JsonValue;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::Date;

use crate::{
    fx::DailyRate,
    util::date,
    verboseln
};

use super::Error;

const CAD_USD_NOON_OBSERVATION: &str = "IEXE0101";
const CAD_USD_DAILY_OBSERVATION: &str = "FXCADUSD";

fn get_fx_json_url(year: u32) -> String {
    let observation: &str = match year >= 2017 {
        true => CAD_USD_DAILY_OBSERVATION,
        false => CAD_USD_NOON_OBSERVATION,
    };
    format!(
        "https://www.bankofcanada.ca/valet/observations/{}/json?start_date={}-01-01&end_date={}-12-31",
        observation, year, year
    )
}

pub struct RateParseResult {
    pub rates: Vec<DailyRate>,
    pub non_fatal_errors: Vec<String>,
}

pub type RateLoadResult = RateParseResult;

pub trait RemoteRateLoader {
    fn get_remote_usd_cad_rates(&mut self, year: u32) -> Result<RateLoadResult, Error>;
}

const JSON_DATE_FORMAT: date::StaticDateFormat = date::STANDARD_DATE_FORMAT;

fn json_value_to_decimal(jv: &JsonValue) -> Result<Decimal, Error> {
    match jv {
        JsonValue::String(v) =>
            Decimal::from_str(v).map_err(|e| e.to_string()),
        JsonValue::Short(v) =>
            Decimal::from_str(&v.to_string()).map_err(|e| e.to_string()),
        JsonValue::Number(v) =>
            Decimal::from_str(&v.to_string()).map_err(|e| e.to_string()),
        v => Err(format!("Value (not a number): {}", v)),
    }
}

fn json_value_to_positive_decimal(jv: &JsonValue) -> Result<Decimal, Error> {
    match json_value_to_decimal(jv) {
        Ok(d) => {
            if d.is_sign_positive() && !d.is_zero() {
                Ok(d)
            } else {
                Err(format!("Value is not positive: {}", d))
            }
        },
        Err(e) => Err(e),
    }
}

fn json_value_to_string(jv: &JsonValue) -> Option<&str> {
    match jv {
        JsonValue::Short(v) => Some(v.as_str()),
        JsonValue::String(s) => Some(s.as_str()),
        _ => None
    }
}

fn parse_rates_json(json_str: &str) -> Result<RateParseResult, Error> {
    let fmt_err = |s: &str| -> Result<_, Error> {
        Err(format!("Error parsing CAD USD rates: {}", s))
    };

    let json_obj = match json::parse(&json_str) {
        Ok(v) => v,
        Err(e) => return fmt_err(&e.to_string()),
    };

    // BoC valet Json schema:
    // {
    //    observations: [
    //      {
    //         d: <date: string, formatted as yyyy-mm-dd>,
    //         "IEXE0101": { v: <value: string encoded float> }, // (before 2017) OR
    //         "FXCADUSD": { v: <value: string encoded float> } // (2017 and later)
    //      }
    //    ]
    // }

    let observations = match &json_obj {
        JsonValue::Object(o) => match o.get("observations") {
            Some(obs) => obs,
            None => return fmt_err("Did not find 'observations'"),
        },
        _ => return fmt_err("Root was not of type object"),
    };

    let mut rates = Vec::new();
    let mut non_fatal_dyn: Vec<String> = vec![];
    let mut non_fatal_static: HashSet<&str> = HashSet::new();

    for v in observations.members() {
        let obs = match v {
            JsonValue::Object(o) => o,
            v => {
                non_fatal_dyn.push(format!("Non-object found in observations: {}", v));
                continue;
            },
        };
        let date_str = match obs.get("d") {
            Some(d) => match json_value_to_string(d) {
                Some(s) => s,
                None => {
                    non_fatal_dyn.push(format!(
                        "Date in rate observation of wrong type: {:?}", d));
                    continue;
                },
            },
            None => {
                non_fatal_static.insert("Rate observation missing date");
                continue;
            },
        };

        let date = match Date::parse(date_str, JSON_DATE_FORMAT) {
            Ok(date) => date,
            Err(e) => {
                non_fatal_dyn.push(format!("Failed to parse date {:?}: {}", date_str, e));
                continue;
            },
        };

        let parse_rate_value = |key: &str| -> Result<Option<Decimal>, Error> {
            let obs_val = match obs.get(key) {
                Some(jv) => match jv{
                    JsonValue::Object(o) => o,
                    v => {
                        return Err(format!("value container was not an object: {}", v));
                    }
                },
                None => return Ok(None),
            };
            // obs_val should look like { v: <value: string encoded float> }
            match obs_val.get("v") {
                Some(jv) => match json_value_to_positive_decimal(jv) {
                    Ok(d) => Ok(Some(d)),
                    Err(e) => Err(e),
                },
                None => Err(format!("No value (\"v\") found")),
            }
        };

        let noon_rate = match parse_rate_value(CAD_USD_NOON_OBSERVATION) {
            Ok(noon_rate_opt) => noon_rate_opt,
            Err(e) => {
                non_fatal_dyn.push(format!("Failed to parse noon rate for {}: {}", date_str, e));
                continue;
            },
        };
        match noon_rate {
            Some(r) => {
                // These rates are specified as eg. 1.3... (USD * rate = CAD)
                rates.push(DailyRate{date: date, foreign_to_local_rate: r});
            },
            None => {
                match parse_rate_value(CAD_USD_DAILY_OBSERVATION) {
                    // These rates are specified as eg. 0.7... (CAD * rate = USD)
                    Ok(daily_rate_top) => match daily_rate_top {
                        Some(r) => { rates.push(DailyRate{
                            date: date,
                            foreign_to_local_rate: dec!(1) / r })
                        },
                        None => (), // No rate found for date.
                    },
                    Err(e) => {
                        non_fatal_dyn.push(format!("Failed to parse daily rate for {}: {}", date_str, e));
                        continue;
                    }
                }
            }
        }
    } // end for json_obj.members()

    let mut non_fatal_errors: Vec<String> =
        Vec::from_iter(non_fatal_static.drain().map(|s| s.to_string()));
    non_fatal_errors.append(&mut non_fatal_dyn);

    Ok(RateParseResult{rates, non_fatal_errors})
}

pub struct JsonRemoteRateLoader {
    // user_error_stream: WriteHandle,
}

impl JsonRemoteRateLoader {
    pub fn new() -> JsonRemoteRateLoader {
        JsonRemoteRateLoader{}
    }
}

impl RemoteRateLoader for JsonRemoteRateLoader {
    fn get_remote_usd_cad_rates(&mut self, year: u32) -> Result<RateLoadResult, Error> {
	    eprint!("Fetching USD/CAD exchange rates for {}\n", year);
        let url = get_fx_json_url(year);
        verboseln!("Fetching {}", url);
        let get_result = reqwest::blocking::get(url);
        let fmt_err = |s: &str| -> Result<_, Error> {
            Err(format!("Error getting CAD USD rates: {}", s))
        };
        let out = match get_result {
            Ok(out) => out,
            Err(e) => return fmt_err(&e.to_string()),
        };
        let out = match out.error_for_status() {
            Ok(o) => o,
            Err(e) => return fmt_err(&format!("status: {:?}", &e.status())),
        };
        let text = match out.text() {
            Ok(t) => t,
            Err(e) => return fmt_err(&e.to_string()),
        };

        parse_rates_json(&text)
    }
}

// Ideally this would be marked as cfg(test), but I want integration
// tests to also have access, so it cannot be marked test-only for it
// to be accessible there.
pub mod pub_testlib {
    use std::collections::HashMap;

    use tracing::{trace};

    use crate::{fx::DailyRate, util::rc::RcRefCell};

    use super::{RateLoadResult, RemoteRateLoader, Error};

    pub struct MockRemoteRateLoader {
        pub remote_year_rates: RcRefCell<HashMap<u32, Vec<DailyRate>>>
    }

    impl RemoteRateLoader for MockRemoteRateLoader {
        fn get_remote_usd_cad_rates(&mut self, year: u32) -> Result<RateLoadResult, Error> {
            trace!(year = year, "MockRemoteRateLoader::get_remote_usd_cad_rates");
            match self.remote_year_rates.borrow().get(&year) {
                Some(rates) =>
                    Ok(RateLoadResult{rates: rates.clone(),
                                      non_fatal_errors: vec![]}),
                None => Err(format!("No rates set for {}", year)),
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::fx::DailyRate;
    use crate::testlib::assert_re;
    use crate::util::date;

    use super::parse_rates_json;

    fn dr(date_str: &str, val: Decimal) -> DailyRate {
        DailyRate{date: date::parse_standard_date(date_str).unwrap(),
                  foreign_to_local_rate: val}
    }
    fn invert(v: Decimal) -> Decimal {
        dec!(1).checked_div(v).unwrap()
    }

    #[test]
    fn test_parse_ok() {
        // Basic empty case
        let result = parse_rates_json("
        {
            \"observations\": [
            ]
        }
        ");
        let r = result.unwrap();
        assert_eq!(r.rates, vec![]);
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);

        // Basic non-empty case (and checking collision precedence)
        let result = parse_rates_json("
        {
           \"observations\": [
             {
                \"d\": \"2023-01-24\",
                \"IEXE0101\": { \"v\": \"1.3456\" },
                \"FXCADUSD\": { \"v\": \"0.7654\" }
             },
             {
                \"d\": \"2023-01-25\",
                \"FXCADUSD\": { \"v\": \"0.7655\" }
             },
             {
                \"d\": \"2023-01-26\",
                \"IEXE0101\": { \"v\": \"1.3457\" }
             }
           ]
        }
        ");
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![
            dr("2023-01-24", dec!(1.3456)),
            dr("2023-01-25", invert(dec!(0.7655))),
            dr("2023-01-26", dec!(1.3457)),
        ]);

        // Longgest precision
        let result = parse_rates_json("
        {
           \"observations\": [
             {
                \"d\": \"2023-01-25\",
                \"FXCADUSD\": { \"v\": \"0.7655555555555555555555555555\" }
             }
           ]
        }
        ");
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![
            dr("2023-01-25", invert(dec!(0.7655555555555555555555555555))),
        ]);

        // Integer value
        let result = parse_rates_json("
        {
           \"observations\": [
             {
                \"d\": \"2023-01-25\",
                \"FXCADUSD\": { \"v\": 1 }
             }
           ]
        }
        ");
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![
            dr("2023-01-25", invert(dec!(1))),
        ]);

        // Float value
        let result = parse_rates_json("
        {
           \"observations\": [
             {
                \"d\": \"2023-01-25\",
                \"FXCADUSD\": { \"v\": 0.7 }
             }
           ]
        }
        ");
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![
            dr("2023-01-25", invert(dec!(0.7))),
        ]);
    }

    #[test]
    fn test_parse_err() {
        let general_err_pattern = "^Error parsing CAD USD rates:";
        let cat = |left: &str, right: &str| -> String {
            left.to_string() + right
        };
        let cat3 = |left: &str, middle: &str, right: &str| -> String {
            left.to_string() + middle + right
        };

        // Case: Invalid json
        let result = parse_rates_json("
        {
        ");
        let e = result.err().unwrap();
        assert_re(general_err_pattern, e.as_str());

        let result = parse_rates_json("
        { \"x\": 123mvl }
        ");
        let e = result.err().unwrap();
        assert_re(general_err_pattern, e.as_str());

        // Case: Invalid root type (non-object)
        let result = parse_rates_json("
        [ { \"observations\": [] } ]
        ");
        let e = result.err().unwrap();
        assert_re(&cat(general_err_pattern, " Root was not of type object"),
                  e.as_str());

        // Case: No observations entry
        let result = parse_rates_json("
        { \"XXXX_observations\": [] }
        ");
        let e = result.err().unwrap();
        assert_re(&cat(general_err_pattern, " Did not find 'observations'"),
                  e.as_str());

        // Case: Invalid observations type (non-array)
        let result = parse_rates_json("
        { \"observations\": { \"foo\": \"bar\" } }
        ");
        // We just iterate members here, which isn't Array specific (for some reason)
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![]);

        let ok_obs_json = "{
            \"d\": \"2023-01-26\",
            \"IEXE0101\": { \"v\": \"1.3457\" }
         }";
         let ok_rate = dr("2023-01-26", dec!(1.3457));

        // Case: Invalid observation entry type (non-object)
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ 1234, ",
            ok_obs_json,
            " ] }"
        ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec!["Non-object found in observations: 1234"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: No date
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"IEXE0101\": { \"v\": \"1.333\" }
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec!["Rate observation missing date"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: Invalid date type (non-string)
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": {},
                \"IEXE0101\": { \"v\": \"1.333\" }
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Date in rate observation of wrong type: Object(Object { store: [] })"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: Invalid date string
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"01-02-2024\",
                \"IEXE0101\": { \"v\": \"1.333\" }
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Failed to parse date \"01-02-2024\": the 'year' component could not be parsed"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: No fx object
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"2024-02-16\"
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        // No error issued here. Assumes there just wasn't a rate on this day.
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: Invalid fx object (non-object)
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"2024-02-16\",
                \"IEXE0101\": \"1.333\"
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Failed to parse noon rate for 2024-02-16: value container was not an object: 1.333"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: No value entry
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"2024-02-16\",
                \"IEXE0101\": {}
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Failed to parse noon rate for 2024-02-16: No value (\"v\") found"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: Invalid value type
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"2024-02-16\",
                \"IEXE0101\": { \"v\": {} }
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Failed to parse noon rate for 2024-02-16: Value (not a number): {}"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: Value precision too high (Decimal::from_str just truncates)
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"2024-02-16\",
                \"IEXE0101\": { \"v\": \"1.11111111111111111111111111111111111111111111111111111111111111111111111111111\" }
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, Vec::new() as Vec<String>);
        assert_eq!(r.rates, vec![
            dr("2024-02-16", dec!(1.1111111111111111111111111111)),
            ok_rate.clone()
            ]);

        // Case: Value unparseable
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [ {
                \"d\": \"2024-02-16\",
                \"IEXE0101\": { \"v\": \"kljsdlfkj\" }
            },",
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Failed to parse noon rate for 2024-02-16: Invalid decimal: unknown character"]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);

        // Case: Multiple errors
        let result = parse_rates_json(&cat3(
            "{ \"observations\": [
            {
                \"d\": \"2024-02-16\",
                \"IEXE0101\": { \"v\": \"kljsdlfkj\" }
            },
            {
                \"d\": \"2024-02-17\",
                \"IEXE0101\": { \"v\": \"kljsdlfkj\" }
            },
            {
                \"d\": \"2024-02-18\",
                \"IEXE0101\": { \"v\": {} }
            },"
            ,
            ok_obs_json,
            " ] }"
            ));
        let r = result.unwrap();
        assert_eq!(r.non_fatal_errors, vec![
            "Failed to parse noon rate for 2024-02-16: Invalid decimal: unknown character",
            "Failed to parse noon rate for 2024-02-17: Invalid decimal: unknown character",
            "Failed to parse noon rate for 2024-02-18: Value (not a number): {}",
            ]);
        assert_eq!(r.rates, vec![ok_rate.clone()]);
    }
}