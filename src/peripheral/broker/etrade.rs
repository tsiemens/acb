use std::path::Path;

use rust_decimal::Decimal;
use time::Date;

use super::BrokerTx;
use crate::{
    portfolio::TxAction,
    util::{
        basic::SError,
        date::{DateRange, StaticDateFormat},
        decimal::parse_large_decimal,
    },
};

use lazy_static::lazy_static;

const ETRADE_ACCOUNT_BROKER_NAME: &str = "E*TRADE";

pub fn new_account(account_number: String) -> super::Account {
    super::Account {
        broker_name: ETRADE_ACCOUNT_BROKER_NAME,
        account_type: String::new(),
        account_num: account_number,
    }
}

struct Searcher {
    bldr: regex::RegexBuilder,
}

impl Searcher {
    pub fn new(pattern: &str) -> Self {
        Searcher {
            bldr: regex::RegexBuilder::new(pattern),
        }
    }

    /// dot_matches_new_line ('s' is the defacto flag name for this)
    pub fn s(&mut self) -> &mut Self {
        self.bldr.dot_matches_new_line(true);
        self
    }

    pub fn get_opt_from(&self, text: &str, group: usize) -> Option<String> {
        let re = self.bldr.build().unwrap();
        match re.captures(text) {
            Some(m) => m.get(group).map(|c| c.as_str().to_string()),
            None => None,
        }
    }

    pub fn get_from(&self, text: &str, group: usize) -> Result<String, SError> {
        let re = self.bldr.build().unwrap();
        match re.captures(text) {
            Some(m) => m
                .get(group)
                .map(|c| c.as_str().to_string())
                .ok_or(format!("Could not get group {group} from {re}")),
            None => Err(format!("Could not find {re}")),
        }
    }

    pub fn get1_opt_from(&self, text: &str) -> Option<String> {
        self.get_opt_from(text, 1)
    }

    pub fn get1_from(&self, text: &str) -> Result<String, SError> {
        self.get_from(text, 1)
    }

    // Convenience alias
    pub fn str1(&self, text: &str) -> Result<String, SError> {
        self.get1_from(text)
    }

    pub fn get1_opt_dec_from(&self, text: &str) -> Result<Option<Decimal>, SError> {
        match self.get1_opt_from(text) {
            Some(val_str) => {
                let d_val =
                    parse_large_decimal(&val_str).map_err(|e| e.to_string())?;
                Ok(Some(d_val))
            }
            None => Ok(None),
        }
    }

    // Convenience alias
    pub fn opt_dec1(&self, text: &str) -> Result<Option<Decimal>, SError> {
        self.get1_opt_dec_from(text)
    }

    pub fn get1_dec_from(&self, text: &str) -> Result<Decimal, SError> {
        let val_str = self.get_from(text, 1)?;
        parse_large_decimal(&val_str).map_err(|e| e.to_string())
    }

    // Convenience alias
    pub fn dec1(&self, text: &str) -> Result<Decimal, SError> {
        self.get1_dec_from(text)
    }
}

fn srch(pattern: &str) -> Searcher {
    Searcher::new(pattern)
}

struct CapturesHelper<'a> {
    pub m: regex::Captures<'a>,
}

impl<'a> CapturesHelper<'a> {
    pub fn new(m: regex::Captures<'a>) -> CapturesHelper<'a> {
        Self { m }
    }

    pub fn opt_group(&self, name: &str) -> Option<&str> {
        self.m.name(name).map(|v| v.as_str())
    }

    pub fn opt_dec_group(&self, name: &str) -> Result<Option<Decimal>, SError> {
        match self.opt_group(name) {
            Some(grp_val) => {
                parse_large_decimal(grp_val).map(|v| Some(v)).map_err(|e| {
                    format!("Error parsing decimal from \"{}\": {}", grp_val, e)
                })
            }
            None => Ok(None),
        }
    }

    pub fn group(&self, name: &str) -> &str {
        self.opt_group(name).unwrap()
    }

    pub fn dec_group(&self, name: &str) -> Result<Decimal, SError> {
        Ok(self.opt_dec_group(name)?.unwrap())
    }
}

fn get_filename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "<unnamed file>".to_string())
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct BenefitEntry {
    pub security: String,

    pub acquire_tx_date: Date,
    pub acquire_settle_date: Date,
    pub acquire_share_price: Decimal,
    pub acquire_shares: Decimal,

    // Sell-to-cover fields for trade confirmation matching
    // (see find_sell_to_cover_trade_set and amend_paired_benefit_sales):
    //
    // - sell_to_cover_shares: Required for matching. This is the primary key
    //   used to find matching trade confirmations (by summing share counts
    //   across candidate trade combos). If None, the benefit is skipped for
    //   sell-to-cover pairing entirely.
    //
    // - sell_to_cover_price: Optional tiebreaker. When multiple trade combos
    //   match by share count, the combo whose weighted-average price is
    //   closest to this value wins. If None, ambiguous matches produce an
    //   error. (Available from benefit PDFs; not available from xlsx.)
    //
    // - sell_to_cover_tx_date, sell_to_cover_settle_date: Populated by
    //   amend_paired_benefit_sales from the matched trade confirmation.
    //   Not expected to be set by parsers (benefit PDFs and xlsx both lack
    //   these; trade confirmation PDFs are the authoritative source).
    //
    // - sell_to_cover_fee: Populated from benefit PDFs if available, otherwise
    //   filled in by amend_paired_benefit_sales from matched trades.
    //
    // All five fields must be Some for sell_to_cover_data() to return
    // Ok(Some(...)). The amend step is responsible for completing the set.
    pub sell_to_cover_tx_date: Option<Date>,
    pub sell_to_cover_settle_date: Option<Date>,
    pub sell_to_cover_price: Option<Decimal>,
    pub sell_to_cover_shares: Option<Decimal>,
    pub sell_to_cover_fee: Option<Decimal>,

    pub plan_note: String,
    pub sell_note: Option<String>,
    pub filename: String,
}

pub struct SellToCoverData {
    pub sell_to_cover_tx_date: Date,
    pub sell_to_cover_settle_date: Date,
    pub sell_to_cover_price: Decimal,
    pub sell_to_cover_shares: Decimal,
    pub sell_to_cover_fee: Decimal,
}

impl BenefitEntry {
    /// Retrieves the sell-to-cover data (if present) for the benefit.
    /// Returns Err if incomplete StC data is populated, and Ok(None) if no
    /// StC data is populated.
    pub fn sell_to_cover_data(&self) -> Result<Option<SellToCoverData>, SError> {
        #[derive(PartialEq, Default, Debug)]
        struct StcOpts {
            sell_to_cover_tx_date: Option<Date>,
            sell_to_cover_settle_date: Option<Date>,
            sell_to_cover_price: Option<Decimal>,
            sell_to_cover_shares: Option<Decimal>,
            sell_to_cover_fee: Option<Decimal>,
        }
        let stc_opts = StcOpts {
            sell_to_cover_tx_date: self.sell_to_cover_tx_date,
            sell_to_cover_settle_date: self.sell_to_cover_settle_date,
            sell_to_cover_price: self.sell_to_cover_price,
            sell_to_cover_shares: self.sell_to_cover_shares,
            sell_to_cover_fee: self.sell_to_cover_fee,
        };
        if stc_opts == StcOpts::default() {
            return Ok(None);
        }

        let err = || {
            format!(
                "Some, but not all, sell-to-cover fields were found for \
                    {} shares of {} aquired on {}. StC {:?}",
                self.acquire_shares, self.security, self.acquire_tx_date, stc_opts
            )
        };

        Ok(Some(SellToCoverData {
            sell_to_cover_tx_date: stc_opts.sell_to_cover_tx_date.ok_or_else(err)?,
            sell_to_cover_settle_date: stc_opts
                .sell_to_cover_settle_date
                .ok_or_else(err)?,
            sell_to_cover_price: stc_opts.sell_to_cover_price.ok_or_else(err)?,
            sell_to_cover_shares: stc_opts.sell_to_cover_shares.ok_or_else(err)?,
            sell_to_cover_fee: stc_opts.sell_to_cover_fee.ok_or_else(err)?,
        }))
    }
}

// Common to all benefit PDFs
#[derive(PartialEq, Debug)]
struct BenefitCommonData {
    #[allow(dead_code)]
    pub employee_id: String,
    #[allow(dead_code)]
    pub account_number: String,
    pub symbol: String,
}

fn parse_benefit_common_data(
    benefit_pdf_text: &str,
) -> Result<BenefitCommonData, SError> {
    let text = benefit_pdf_text;
    Ok(BenefitCommonData {
        employee_id: srch(r"Employee ID:\s*(\d+)").str1(text)?,
        account_number: srch(r"Account (?:Number|Stock Plan \(\S+\) -)\s*(\d+)")
            .str1(text)?,
        symbol: srch(r"Company Name\s*\(Symbol\)*.*\(([A-Za-z\.]+)\)")
            .s()
            .str1(text)?,
    })
}

pub const ETRADE_DASH_DATE_FORMAT: StaticDateFormat =
    time::macros::format_description!("[month]-[day]-[year]");
pub const ETRADE_SLASH_DATE_FORMAT: StaticDateFormat =
    time::macros::format_description!("[month]/[day]/[year]");
lazy_static! {
    static ref ETRADE_SHORT_SLASH_DATE_RE: regex::Regex =
        regex::Regex::new(r"(\d+/\d+)/(\d+)").unwrap();
}

/// This is required, because the default parse lib doesn't like two-digit years,
/// even if it lets you specify [year repr:last_two], it will just fail, claiming
/// there isn't enough information to construct the Date.
///
/// This function just cheats, and assumes we're in the 21st century.
fn parse_short_year_date(date_str: &str) -> Result<Date, SError> {
    match ETRADE_SHORT_SLASH_DATE_RE.captures(date_str) {
        Some(m) => {
            let long_date = format!(
                "{}/20{}",
                m.get(1).unwrap().as_str(),
                m.get(2).unwrap().as_str()
            );
            Date::parse(&long_date, ETRADE_SLASH_DATE_FORMAT)
                .map_err(|e| e.to_string())
        }
        None => Err(format!(
            "Failed to parse date. {} did not match \
                            {:?}",
            date_str, *ETRADE_SHORT_SLASH_DATE_RE
        )),
    }
}

struct RsuData {
    pub common_benefit_data: BenefitCommonData,
    pub release_date: Date,
    pub award_number: String,
    pub shares_released: Decimal,
    pub shares_sold: Decimal,
    #[allow(dead_code)]
    pub shares_issued: Decimal,
    pub fmv_per_share: Decimal,
    pub sale_price_per_share: Decimal,
    #[allow(dead_code)]
    pub market_value: Decimal,
    #[allow(dead_code)]
    pub total_sale_price: Decimal,
    #[allow(dead_code)]
    pub total_tax: Decimal,
    #[allow(dead_code)]
    pub fee: Decimal,
    #[allow(dead_code)]
    pub cash_leftover: Decimal,
}

fn parse_rsu_data(rsu_pdf_text: &str) -> Result<RsuData, SError> {
    let text = rsu_pdf_text;

    Ok(RsuData {
        common_benefit_data: parse_benefit_common_data(text)?,
        release_date: Date::parse(
            &srch(r"Release Date\s*(\d+-\d+-\d+)").str1(text)?,
            ETRADE_DASH_DATE_FORMAT,
        )
        .map_err(|e| e.to_string())?,
        award_number: srch(r"Award Number\s*(R\d+)").str1(text)?,
        shares_released: extract_numeric("Shares Released", false, text)?,
        shares_sold: extract_numeric("Shares Sold", true, text)?,
        shares_issued: extract_numeric("Shares Issued", false, text)?,
        fmv_per_share: extract_currency("Market Value Per Share", false, text)?,
        sale_price_per_share: extract_currency("Sale Price Per Share", false, text)?,
        market_value: extract_currency("Market Value", false, text)?,
        total_sale_price: extract_currency("Total Sale Price", false, text)?,
        total_tax: extract_currency("Total Tax", false, text)?,
        fee: extract_currency("Fee", true, text)?,
        cash_leftover: extract_currency("Total Due Participant", false, text)?,
    })
}

fn parse_rsu_entry(
    rsu_pdf_text: &str,
    filepath: &Path,
) -> Result<BenefitEntry, SError> {
    let rsu_data = parse_rsu_data(rsu_pdf_text)?;

    Ok(BenefitEntry {
        security: rsu_data.common_benefit_data.symbol,
        // The FMV is for the release date, so treat that as the tx date.
        acquire_tx_date: rsu_data.release_date,
        // There is no way to know the settlement date in RSU distributions.
        // Since they are never near the year-end boundary, just use the release date.
        acquire_settle_date: rsu_data.release_date,
        acquire_share_price: rsu_data.fmv_per_share,
        acquire_shares: rsu_data.shares_released,

        // The sell-to-cover date is almost always a day or two after the release
        // date. This needs to be looked up separately if we want an accurate
        // USD/CAD exchange rate.
        sell_to_cover_tx_date: None,
        sell_to_cover_settle_date: None,
        sell_to_cover_price: Some(rsu_data.sale_price_per_share),
        sell_to_cover_shares: Some(rsu_data.shares_sold),
        sell_to_cover_fee: Some(rsu_data.fee),

        plan_note: format!("RSU {}", rsu_data.award_number),
        sell_note: None,
        filename: get_filename(filepath),
    })
}

#[derive(PartialEq, Debug)]
struct EsoGrantData {
    grant_number: u64,
    exercise_fmv: Decimal,
    shares_exercised: Decimal,
    sale_price: Decimal,
    fee: Decimal,
}

#[derive(PartialEq, Debug)]
struct EsoData {
    pub common_benefit_data: BenefitCommonData,
    pub exercise_type: String,
    pub exercise_date: Date,
    pub shares_sold: Decimal,

    pub grants: Vec<EsoGrantData>,
}

/// Attempts to look for all rows within text that are of the format:
/// KEY: VAL_PAT VAL_PAT?
fn search_for_rows(
    key: &str,
    val_pat: &str,
    text: &str,
) -> Result<Vec<String>, SError> {
    let main_re = regex::Regex::new(&format!(
        r"{key}(?:\s+(?P<rowvalue1>{val_pat})(?:\s+(?P<rowvalue2>{val_pat}))?)"
    ))
    .unwrap();

    let mut vals = Vec::<String>::new();
    for m in main_re.captures_iter(text) {
        vals.push(m.name("rowvalue1").unwrap().as_str().to_string());
        if let Some(v2) = m.name("rowvalue2") {
            vals.push(v2.as_str().to_string());
        }
    }
    if vals.is_empty() {
        Err(format!("Could not find {main_re:?}"))
    } else {
        Ok(vals)
    }
}

fn build_value_pattern(dollar_prefix: bool, parens: bool) -> String {
    let prefix = if dollar_prefix { r"\$" } else { "" };
    let p_beg = if parens { r"\(" } else { "" };
    let p_end = if parens { r"\)" } else { "" };
    return format!(r"{p_beg}{prefix}([\d,\.]+){p_end}");
}

fn remove_symbols(s: &str) -> String {
    s.chars().filter(|c| *c != '$' && *c != '(' && *c != ')').collect()
}

fn search_for_dec_rows(
    key: &str,
    dollar_prefix: bool,
    parens: bool,
    text: &str,
) -> Result<Vec<Decimal>, SError> {
    let pattern = &build_value_pattern(dollar_prefix, parens);
    let strs = search_for_rows(key, pattern, text)?;
    let mut decs = Vec::<Decimal>::with_capacity(strs.len());
    for s in strs {
        let sanitized_s = remove_symbols(&s);
        decs.push(parse_large_decimal(&sanitized_s).map_err(|e| {
            format!("Decimal error in \"{sanitized_s}\" on \"{key}\" row: {e}")
        })?);
    }
    Ok(decs)
}

fn extract_dec_common(
    key: &str,
    dollar_prefix: bool,
    parens: bool,
    text: &str,
) -> Result<Decimal, SError> {
    let rows = search_for_dec_rows(key, dollar_prefix, parens, text)
        .map_err(|e| format!("Could not find \"{key}\" decimal value: {e}"))?;

    if rows.len() != 1 {
        return Err(format!(
            "Only expected a single \"{key}\" value, but found: {:?}",
            rows
        ));
    }

    return Ok(rows[0]);
}

fn extract_numeric(key: &str, parens: bool, text: &str) -> Result<Decimal, SError> {
    return extract_dec_common(key, false, parens, text);
}

fn extract_currency(key: &str, parens: bool, text: &str) -> Result<Decimal, SError> {
    return extract_dec_common(key, true, parens, text);
}

fn parse_eso_data(eso_pdf_text: &str) -> Result<EsoData, SError> {
    let text = eso_pdf_text;

    const BODY_START: &str = "Exercise Details";
    const BODY_END: &str = "Exercise Date|EMPLOYEE STOCK PLAN EXERCISE CONFIRMATION";
    let body_pat = &format!(r"^(.*)({BODY_START}.*(?:{BODY_END})).*$");
    let body_m_res = regex::RegexBuilder::new(body_pat)
        .dot_matches_new_line(true)
        .build()
        .unwrap()
        .captures(text);
    let (header, body) = if let Some(body_m) = body_m_res {
        (
            body_m.get(1).unwrap().as_str(),
            body_m.get(2).unwrap().as_str(),
        )
    } else {
        return Err("Unable to parse exercise details".to_string());
    };

    let grant_indicies: Vec<String> = regex::Regex::new(r"Grant (\d+)")
        .unwrap()
        .find_iter(body)
        .map(|m| m.as_str().to_string())
        .collect();
    tracing::debug!("parse_eso_data grants: {:?}", grant_indicies);

    let grant_numbers: Vec<u64> = search_for_rows("Grant Number", r"\d+", body)?
        .into_iter()
        .map(|s| s.parse::<u64>().unwrap_or_default())
        .collect();
    let grant_exercise_fmvs =
        search_for_dec_rows("Exercise Market Value", true, false, body)?;
    let grant_shares_exercised =
        search_for_dec_rows("Shares Exercised", false, false, body)?;
    let grant_sale_prices = search_for_dec_rows("Sale Price", true, false, body)?;
    let grant_fees = search_for_dec_rows("Comission/Fee", true, false, body)?;

    let mut grants = Vec::with_capacity(grant_indicies.len());
    for (((((_, num), fmv), shares), s_price), fee) in grant_indicies
        .iter()
        .zip(grant_numbers)
        .zip(grant_exercise_fmvs)
        .zip(grant_shares_exercised)
        .zip(grant_sale_prices)
        .zip(grant_fees)
    {
        grants.push(EsoGrantData {
            grant_number: num,
            exercise_fmv: fmv,
            shares_exercised: shares,
            sale_price: s_price,
            fee: fee,
        });
    }

    Ok(EsoData {
        common_benefit_data: parse_benefit_common_data(text)?,
        exercise_type: srch(r"Exercise Type:\s+(.*)\s+Registration").str1(text)?,
        exercise_date: Date::parse(
            &srch(r"Exercise Date:\s+(\d+/\d+/\d+)").str1(text)?,
            ETRADE_SLASH_DATE_FORMAT,
        )
        .map_err(|e| e.to_string())?,
        shares_sold: extract_numeric("Shares Sold", false, header)?,
        grants: grants,
    })
}

/// Parses BenefitEntries out of exercies stock options confirmations.
/// Each form can contain of multiple grants exercises, and each will yield
/// a separate benefit entry.
/// Due to how some attributes are consolidated (sold shares, for example),
/// some parts are added into just the last BenefitEntry.
fn parse_eso_entries(
    eso_pdf_text: &str,
    filepath: &Path,
) -> Result<Vec<BenefitEntry>, SError> {
    let eso_data = parse_eso_data(eso_pdf_text)?;

    let mut entries = Vec::with_capacity(eso_data.grants.len());
    let last_grant = eso_data
        .grants
        .last()
        .ok_or_else(|| format!("No exercised grants found in {filepath:?}"))?;

    // Decimal doesn't implement Sum, so we have to manually accumulate it.
    let fee_sum = eso_data.grants.iter().fold(Decimal::ZERO, |acc, g| acc + g.fee);

    for (i, grant) in eso_data.grants.iter().enumerate() {
        if grant.sale_price != last_grant.sale_price {
            return Err(format!(
                "Non-equal ESO sale prices {} and {}",
                grant.sale_price, last_grant.sale_price
            ));
        }

        let is_last = i == eso_data.grants.len() - 1;
        entries.push(BenefitEntry {
            security: eso_data.common_benefit_data.symbol.clone(),
            acquire_tx_date: eso_data.exercise_date,
            acquire_settle_date: eso_data.exercise_date,
            acquire_share_price: grant.exercise_fmv,
            acquire_shares: grant.shares_exercised,
            sell_to_cover_tx_date: if is_last {
                Some(eso_data.exercise_date)
            } else {
                None
            },
            sell_to_cover_settle_date: if is_last {
                Some(eso_data.exercise_date)
            } else {
                None
            },
            sell_to_cover_price: if is_last {
                Some(grant.sale_price)
            } else {
                None
            },
            sell_to_cover_shares: if is_last {
                Some(eso_data.shares_sold)
            } else {
                None
            },
            sell_to_cover_fee: if is_last { Some(fee_sum) } else { None },
            plan_note: format!("Option Grant {}", grant.grant_number),
            sell_note: Some(eso_data.exercise_type.clone()),
            filename: get_filename(filepath),
        });
    }
    Ok(entries)
}

struct EsppData {
    pub common_benefit_data: BenefitCommonData,
    pub purchase_date: Date,
    pub shares_purchased: Decimal,
    pub fmv_per_share: Decimal,
    #[allow(dead_code)]
    pub purchase_price_per_share: Decimal,
    #[allow(dead_code)]
    pub total_price: Decimal,
    #[allow(dead_code)]
    pub total_value: Decimal,
    #[allow(dead_code)]
    pub taxable_gain: Decimal,
    #[allow(dead_code)]
    pub market_value_at_grant: Decimal,

    #[allow(dead_code)]
    pub total_tax: Option<Decimal>,
    pub shares_sold: Option<Decimal>,
    pub sale_price_per_share: Option<Decimal>,
    #[allow(dead_code)]
    pub total_sale_price: Option<Decimal>,
    pub fee: Option<Decimal>,
    #[allow(dead_code)]
    pub cash_leftover: Option<Decimal>,
}

fn parse_espp_data(espp_pdf_text: &str) -> Result<EsppData, SError> {
    let text = espp_pdf_text;

    Ok(EsppData {
        common_benefit_data: parse_benefit_common_data(text)?,
        purchase_date: Date::parse(
            &srch(r"Purchase Date\s*(\d+-\d+-\d+)").str1(text)?,
            ETRADE_DASH_DATE_FORMAT,
        )
        .map_err(|e| e.to_string())?,
        shares_purchased: srch(r"Shares Purchased\s*(\d+\.\d+)").dec1(text)?,
        fmv_per_share: srch(r"Purchase Value per Share\s*\$(\d+\.\d+)")
            .dec1(text)?,
        purchase_price_per_share: srch(
            r"Purchase Price per Share\s*\([^\)]*\)\s*\$(\d+\.\d+)",
        )
        .s()
        .dec1(text)?,
        total_price: srch(r"Total Price\s*\(\$([\d,]+\.\d+)\)").dec1(text)?,
        total_value: srch(r"Total Value\s*\$([\d,]+\.\d+)").dec1(text)?,
        taxable_gain: srch(r"Taxable Gain\s*\$([\d,]+\.\d+)").dec1(text)?,
        market_value_at_grant: srch(r"Market Value\s*\$([\d,]+\.\d+)").dec1(text)?,

        total_tax: srch(r"Total Taxes Collected at purchase\s\(\$([\d,]+\.\d+)\)")
            .opt_dec1(text)?,
        shares_sold: srch(r"Shares Sold to Cover Taxes\s*(\d+\.\d+)")
            .opt_dec1(text)?,
        sale_price_per_share: srch(
            r"Sale Price for Shares Sold to Cover Taxes\s*\$(\d+\.\d+)",
        )
        .opt_dec1(text)?,
        total_sale_price: srch(r"Value Of Shares Sold\s\$([\d,]+\.\d+)")
            .opt_dec1(text)?,
        fee: srch(r"Fees\s*\(\$(\d+\.\d+)").opt_dec1(text)?,
        cash_leftover: srch(r"Amount in Excess of Tax Due\s\$(\d+\.\d+)")
            .opt_dec1(text)?,
    })
}

fn parse_espp_entry(
    espp_pdf_text: &str,
    filepath: &Path,
) -> Result<BenefitEntry, SError> {
    let espp_data = parse_espp_data(espp_pdf_text)?;

    Ok(BenefitEntry {
        security: espp_data.common_benefit_data.symbol,
        acquire_tx_date: espp_data.purchase_date,
        // There is no way to know the settlement date in ESPP distributions.
        // Since they are never near the year-end boundary, just use the purchase
        // date.
        acquire_settle_date: espp_data.purchase_date,
        acquire_share_price: espp_data.fmv_per_share,
        acquire_shares: espp_data.shares_purchased,
        // The sell-to-cover date is almost always a day or two after the release
        // date. This needs to be looked up separately if we want an accurate
        // USD/CAD exchange rate.
        sell_to_cover_tx_date: None,
        sell_to_cover_settle_date: None,
        sell_to_cover_price: espp_data.sale_price_per_share,
        sell_to_cover_shares: espp_data.shares_sold,
        sell_to_cover_fee: espp_data.fee,

        plan_note: "ESPP".to_string(),
        sell_note: None,
        filename: get_filename(filepath),
    })
}

/// Trade confirmation form before Morgan Stanley aquired ETRADE
/// (mid 2023 and before)
fn parse_pre_ms_2023_trade_confirmations(
    pdf_text: &str,
    filepath: &Path,
) -> Result<Vec<BrokerTx>, SError> {
    let account_number = srch(r"Account\s+Number:\s*(\S+)\s").str1(pdf_text)?;

    let trade_pat = regex::Regex::new(concat!(
        r"(?P<txdate>\d+/\d+/\d+)\s+(?P<sdate>\d+/\d+/\d+)\s+",
        r"(?P<mkt>\d+)\s*(?P<cpt>\d+)\s+",
        r"(?P<sym>\S+)\s+(?P<act>\S+)\s+(?P<nshares>\d+)\s+\$(?P<price>\d+\.\d+)[^\n]*\n",
        r"[^\n]*(COMMISSION\s+\$(?P<commission>\d+\.\d+)[^\n]*\n)?",
        r"[^\n]*(FEE\s+\$(?P<fee>\d+\.\d+)[^\n]*\n)?",
        r"[^\n]*NET\s+AMOUNT"),
    ).unwrap();

    let mut txs = Vec::<BrokerTx>::new();
    for (i, m) in trade_pat.captures_iter(pdf_text).enumerate() {
        let h = CapturesHelper::new(m);

        txs.push(BrokerTx {
            security: h.group("sym").to_string(),
            trade_date: parse_short_year_date(h.group("txdate")).map_err(|e| {
                format!("Date parse error in {}: {}", h.group("txdate"), e)
            })?,
            settlement_date: parse_short_year_date(h.group("sdate")).map_err(
                |e| format!("Date parse error in {}: {}", h.group("sdate"), e),
            )?,
            trade_date_and_time: h.group("txdate").to_string(),
            settlement_date_and_time: h.group("sdate").to_string(),
            action: TxAction::try_from(h.group("act"))?,
            amount_per_share: h.dec_group("price")?,
            num_shares: h.dec_group("nshares")?,
            commission: h.opt_dec_group("commission")?.unwrap_or(Decimal::ZERO)
                + h.opt_dec_group("fee")?.unwrap_or(Decimal::ZERO),
            currency: crate::portfolio::Currency::usd(),
            memo: String::new(),
            exchange_rate: None,
            affiliate: crate::portfolio::Affiliate::default(),
            row_num: (i + 1).try_into().unwrap(),
            account: new_account(account_number.clone()),
            sort_tiebreak: None,
            filename: Some(get_filename(filepath)),
        });
    }
    Ok(txs)
}

/// Trade confirmation form after Morgan Stanley aquired ETRADE
/// (mid 2023 and later)
///
/// Note that these PDFs have to be parsed by pypdf. This is handled by the
/// automatic parsing in get_pages_text_from_path though.
fn parse_post_ms_2023_trade_confirmation(
    pdf_text: &str,
    filepath: &Path,
) -> Result<BrokerTx, SError> {
    let account_number = srch(r"Account\s+Number:\s*(\S+)\s").str1(pdf_text)?;

    let trade_pat = regex::Regex::new(concat!(
        r"Trade\s+Date\s+Settlement\s+Date\s+Quantity\s+Price\s+Settlement\s+Amount\s+",
        r"(?P<txdate>\d+/\d+/\d+)\s+(?P<sdate>\d+/\d+/\d+)\s+(?P<nshares>\d+)\s+",
        r"(?P<price>\d+\.\d+)\s+",
        r"Transaction\s+Type:\s*(?P<act>\S.*\S)\s*",
        r"Description.*\n.*ISIN:\s*(?P<sym>\S+)",
        r"([\s\S]*Commission\s+\$(?P<commission>\d+\.\d+))?",
        r"([\s\S]*Transaction\s+Fee\s+\$(?P<fee>\d+\.\d+))?")
    ).unwrap();

    if let Some(m) = trade_pat.captures(pdf_text) {
        let h = CapturesHelper::new(m);

        Ok(BrokerTx {
            security: h.group("sym").to_string(),
            trade_date: Date::parse(h.group("txdate"), ETRADE_SLASH_DATE_FORMAT)
                .map_err(|e| {
                    format!("Date parse error in {}: {}", h.group("txdate"), e)
                })?,
            settlement_date: Date::parse(h.group("sdate"), ETRADE_SLASH_DATE_FORMAT)
                .map_err(|e| {
                    format!("Date parse error in {}: {}", h.group("sdate"), e)
                })?,
            trade_date_and_time: h.group("txdate").to_string(),
            settlement_date_and_time: h.group("sdate").to_string(),
            action: TxAction::try_from(h.group("act"))?,
            amount_per_share: h.dec_group("price")?,
            num_shares: h.dec_group("nshares")?,
            commission: h.opt_dec_group("commission")?.unwrap_or(Decimal::ZERO)
                + h.opt_dec_group("fee")?.unwrap_or(Decimal::ZERO),
            currency: crate::portfolio::Currency::usd(),
            memo: String::new(),
            exchange_rate: None,
            affiliate: crate::portfolio::Affiliate::default(),
            row_num: 1,
            account: new_account(account_number.clone()),
            sort_tiebreak: None,
            filename: Some(get_filename(filepath)),
        })
    } else {
        parse_post_ms_2023_trade_confirmation_alt(pdf_text, filepath)
    }
}

// An alternative parser for post-MS-2023 trade confirmations, though it's
// not clear yet if this applies to all documents post ~2025-Q4.
// The formatting for these after extracting the text is quite weird, though the
// rendered PDF looks essentially the same as it did before.
fn parse_post_ms_2023_trade_confirmation_alt(
    pdf_text: &str,
    filepath: &Path,
) -> Result<BrokerTx, SError> {
    // Step 1: Remove extraneous text that confuses word boundaries, etc.
    // eg. the page 1 of 2, 2 of 2, etc.
    // Find the final one, which should be 2 of 2, or something
    // (almost always 2 of 2). Then we can use the known number of pages to
    // adjust the text extraction accordingly.

    let final_page_re = regex::Regex::new(r"(\d+)\s+of\s+(\d+)").unwrap();
    let mut last_total_pages = 0;
    for m in final_page_re.captures_iter(pdf_text) {
        let page_num: usize = m.get(1).unwrap().as_str().parse().unwrap_or(0);
        let total_pages: usize = m.get(2).unwrap().as_str().parse().unwrap_or(0);
        if page_num == total_pages {
            last_total_pages = total_pages;
        }
    }

    let mut cleaned_text = pdf_text.to_string();
    tracing::debug!(
        "parse_post_ms_2023_trade_confirmation_alt: cleaning {} page indicators",
        last_total_pages
    );
    for i in 1..=last_total_pages {
        let page_marker = format!("{} of {}", i, last_total_pages);
        cleaned_text = cleaned_text.replace(&page_marker, "");
    }

    // Step 2:
    // Individually search for each field as a separate regex, since they might
    // be ordered weirdly.
    let symbol_re = regex::Regex::new(r"\bISIN:\s*(?P<symbol>\S+)").unwrap();
    let values_line_re = regex::Regex::new(
        r"(?P<txdate>\d+/\d+/\d+)\s+(?P<sdate>\d+/\d+/\d+)\s+(?P<nshares>\d+)\s+(?P<price>\d+\.\d+)\s+"
    )
    .unwrap();
    let action_re =
        regex::Regex::new(r"Transaction Type:\s*(?P<action>\S.*\S)").unwrap();
    let commission_re =
        regex::Regex::new(r"Commission\s*\$(?P<commission>\S+)").unwrap();
    let fee_re = regex::Regex::new(r"Transaction Fee\s*\$(?P<fee>\S+)").unwrap();
    let account_re = regex::Regex::new(r"(?P<account>\d+-XXX\d+-\d+)").unwrap();

    let do_match_opt = |re: &regex::Regex| -> Result<Option<String>, SError> {
        if let Some(m) = re.captures(&cleaned_text) {
            Ok(Some(m.get(1).unwrap().as_str().to_string()))
        } else {
            Ok(None)
        }
    };

    let do_match = |re: &regex::Regex, field_name: &str| -> Result<String, SError> {
        if let Some(m) = re.captures(&cleaned_text) {
            Ok(m.name(field_name).unwrap().as_str().to_string())
        } else {
            Err(format!("Could not find field {}", field_name))
        }
    };

    let do_match_dec_opt =
        |re: &regex::Regex, field_name: &str| -> Result<Option<Decimal>, SError> {
            if let Some(m) = re.captures(&cleaned_text) {
                let val_str = m.get(1).unwrap().as_str();
                let val_dec = val_str.parse::<Decimal>().map_err(|e| {
                    format!("Decimal parse error in field {}: {}", field_name, e)
                })?;
                Ok(Some(val_dec))
            } else {
                Ok(None)
            }
        };

    let values_m = CapturesHelper::new(
        values_line_re
            .captures(&cleaned_text)
            .ok_or_else(|| "Could not find date, price line".to_string())?,
    );

    let txdate_str = values_m.group("txdate").to_string();
    let sdate_str = values_m.group("sdate").to_string();

    Ok(BrokerTx {
        security: do_match(&symbol_re, "symbol")?,
        trade_date: Date::parse(&txdate_str, ETRADE_SLASH_DATE_FORMAT)
            .map_err(|e| format!("Date parse error in {}: {}", txdate_str, e))?,
        settlement_date: Date::parse(&sdate_str, ETRADE_SLASH_DATE_FORMAT)
            .map_err(|e| format!("Date parse error in {}: {}", sdate_str, e))?,
        trade_date_and_time: txdate_str.clone(),
        settlement_date_and_time: sdate_str.clone(),
        action: TxAction::try_from(do_match(&action_re, "action")?.as_str())?,
        amount_per_share: values_m.dec_group("price")?,
        num_shares: values_m.dec_group("nshares")?,
        commission: do_match_dec_opt(&commission_re, "commission")?
            .unwrap_or(Decimal::ZERO)
            + do_match_dec_opt(&fee_re, "fee")?.unwrap_or(Decimal::ZERO),
        currency: crate::portfolio::Currency::usd(),
        memo: String::new(),
        exchange_rate: None,
        affiliate: crate::portfolio::Affiliate::default(),
        row_num: 1,
        account: new_account(
            do_match_opt(&account_re)?.unwrap_or("UNKNOWN".to_string()),
        ),
        sort_tiebreak: None,
        filename: Some(get_filename(filepath)),
    })
}

pub enum EtradePdfContent {
    BenefitConfirmation(Vec<BenefitEntry>),
    TradeConfirmation(Vec<BrokerTx>),
}

lazy_static! {
    static ref RSU_PATTERN: regex::Regex =
        regex::Regex::new(r"STOCK\s+PLAN\s+RELEASE\s+CONFIRMATION").unwrap();
    static ref ESO_PATTERN: regex::Regex =
        regex::Regex::new(r"STOCK\s+PLAN\s+EXERCISE\s+CONFIRMATION").unwrap();
    static ref ESPP_PATTERN: regex::Regex =
        regex::Regex::new(r"Plan\s*(2014|ESP2)").unwrap();
    static ref PRE_MS_2023_TRADE_CONF_PATTERN: regex::Regex =
        regex::Regex::new(r"TRADE\s*CONFIRMATION").unwrap();
    static ref POST_MS_2023_TRADE_CONF_PATTERN: regex::Regex =
        regex::Regex::new(r"This\s+transaction\s+is\s+confirmed").unwrap();
}

pub fn parse_pdf_text(
    etrade_pdf_text: &str,
    filepath: &Path,
) -> Result<EtradePdfContent, SError> {
    if RSU_PATTERN.is_match(etrade_pdf_text) {
        tracing::trace!("parse_pdf_text: {filepath:?} is RSU");
        Ok(EtradePdfContent::BenefitConfirmation(vec![
            parse_rsu_entry(etrade_pdf_text, filepath)?,
        ]))
    } else if ESO_PATTERN.is_match(etrade_pdf_text) {
        Ok(EtradePdfContent::BenefitConfirmation(parse_eso_entries(
            etrade_pdf_text,
            filepath,
        )?))
    } else if ESPP_PATTERN.is_match(etrade_pdf_text) {
        Ok(EtradePdfContent::BenefitConfirmation(vec![
            parse_espp_entry(etrade_pdf_text, filepath)?,
        ]))
    } else if PRE_MS_2023_TRADE_CONF_PATTERN.is_match(etrade_pdf_text) {
        Ok(EtradePdfContent::TradeConfirmation(
            parse_pre_ms_2023_trade_confirmations(etrade_pdf_text, filepath)?,
        ))
    } else if POST_MS_2023_TRADE_CONF_PATTERN.is_match(etrade_pdf_text) {
        Ok(EtradePdfContent::TradeConfirmation(vec![
            parse_post_ms_2023_trade_confirmation(etrade_pdf_text, filepath)?,
        ]))
    } else {
        Err("Cannot categorize layout of PDF".to_string())
    }
}

// MARK: BenefitHistory xlsx parsing

/// Parses a date in "DD-MON-YYYY" format (e.g. "15-FEB-2017").
fn parse_etrade_dmy_date(date_str: &str) -> Result<Date, SError> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Expected DD-MON-YYYY date format, got \"{date_str}\""
        ));
    }
    let day: u8 =
        parts[0].parse().map_err(|e| format!("Bad day in \"{date_str}\": {e}"))?;
    let month = match parts[1].to_uppercase().as_str() {
        "JAN" => time::Month::January,
        "FEB" => time::Month::February,
        "MAR" => time::Month::March,
        "APR" => time::Month::April,
        "MAY" => time::Month::May,
        "JUN" => time::Month::June,
        "JUL" => time::Month::July,
        "AUG" => time::Month::August,
        "SEP" => time::Month::September,
        "OCT" => time::Month::October,
        "NOV" => time::Month::November,
        "DEC" => time::Month::December,
        _ => {
            return Err(format!("Unknown month \"{}\" in \"{date_str}\"", parts[1]))
        }
    };
    let year: i32 =
        parts[2].parse().map_err(|e| format!("Bad year in \"{date_str}\": {e}"))?;
    Date::from_calendar_date(year, month, day).map_err(|e| e.to_string())
}

/// Parses a dollar-prefixed string like "$100.67" into a Decimal.
fn parse_dollar_str(s: &str) -> Result<Decimal, SError> {
    let stripped = s.trim().trim_start_matches('$');
    parse_large_decimal(stripped)
        .map_err(|e| format!("Failed to parse dollar value \"{s}\": {e}"))
}

#[cfg(feature = "xlsx_read")]
fn parse_espp_benefits_from_sheet(
    range: &calamine::Range<calamine::Data>,
    filename: &str,
) -> Result<Vec<BenefitEntry>, SError> {
    use crate::peripheral::excel::SheetReader;

    let mut rows = range.rows();
    let mut reader = SheetReader::new(&mut rows).map_err(|e| e.to_string())?;
    let mut benefits = Vec::new();

    for (i, row) in rows.enumerate() {
        reader.set_row(row, i + 2);

        let record_type =
            reader.get_str("Record Type").map_err(|e| e.to_string())?;
        if record_type != "Purchase" {
            continue;
        }

        let symbol = reader.get_str("Symbol").map_err(|e| e.to_string())?;
        let purchase_date_str =
            reader.get_str("Purchase Date").map_err(|e| e.to_string())?;
        let purchase_date = parse_etrade_dmy_date(&purchase_date_str)?;

        let fmv_str =
            reader.get_str("Purchase Date FMV").map_err(|e| e.to_string())?;
        let fmv = parse_dollar_str(&fmv_str)?;

        let purchased_qty =
            reader.get_dec("Purchased Qty.").map_err(|e| e.to_string())?;
        let tax_shares = reader
            .get_opt_dec("Tax Collection Shares")
            .map_err(|e| e.to_string())?;

        let sell_to_cover_shares = match tax_shares {
            Some(s) if !s.is_zero() => Some(s),
            _ => None,
        };

        benefits.push(BenefitEntry {
            security: symbol,
            acquire_tx_date: purchase_date,
            acquire_settle_date: purchase_date,
            acquire_share_price: fmv,
            acquire_shares: purchased_qty,
            sell_to_cover_tx_date: None,
            sell_to_cover_settle_date: None,
            sell_to_cover_price: None,
            sell_to_cover_shares,
            sell_to_cover_fee: None,
            plan_note: "ESPP".to_string(),
            sell_note: None,
            filename: filename.to_string(),
        });
    }

    Ok(benefits)
}

#[cfg(feature = "xlsx_read")]
fn parse_rsu_benefits_from_sheet(
    range: &calamine::Range<calamine::Data>,
    filename: &str,
) -> Result<Vec<BenefitEntry>, SError> {
    use crate::peripheral::excel::SheetReader;

    let mut rows = range.rows();
    let mut reader = SheetReader::new(&mut rows).map_err(|e| e.to_string())?;

    let mut benefits = Vec::new();
    let mut current_symbol = String::new();
    let mut current_grant_number = String::new();

    // Collect rows so we can look ahead for Tax Withholding after Vest Schedule.
    let all_rows: Vec<&[calamine::Data]> = rows.collect();

    let mut i = 0;
    while i < all_rows.len() {
        let row = all_rows[i];
        reader.set_row(row, i + 2);

        let record_type =
            reader.get_str("Record Type").map_err(|e| e.to_string())?;

        match record_type.as_str() {
            "Grant" => {
                current_symbol =
                    reader.get_str("Symbol").map_err(|e| e.to_string())?;
                current_grant_number =
                    reader.get_str("Grant Number").map_err(|e| e.to_string())?;
            }
            "Vest Schedule" => {
                let released_qty = reader
                    .get_opt_dec("Released Qty")
                    .map_err(|e| e.to_string())?
                    .unwrap_or(Decimal::ZERO);

                if released_qty.is_zero() {
                    i += 1;
                    continue;
                }

                let vest_date_str =
                    reader.get_str("Vest Date").map_err(|e| e.to_string())?;
                let vest_date =
                    Date::parse(&vest_date_str, ETRADE_SLASH_DATE_FORMAT).map_err(
                        |e| format!("Bad vest date \"{vest_date_str}\": {e}"),
                    )?;

                // Look ahead for the Tax Withholding row to get Taxable Gain.
                let taxable_gain = if i + 1 < all_rows.len() {
                    reader.set_row(all_rows[i + 1], i + 3);
                    let next_type =
                        reader.get_str("Record Type").map_err(|e| e.to_string())?;
                    if next_type == "Tax Withholding" {
                        let gain = reader
                            .get_dec("Taxable Gain")
                            .map_err(|e| e.to_string())?;
                        i += 1; // consume the Tax Withholding row
                        gain
                    } else {
                        return Err(format!(
                            "Expected Tax Withholding row after Vest Schedule \
                             for {} on {vest_date_str}, got \"{next_type}\"",
                            current_grant_number
                        ));
                    }
                } else {
                    return Err(format!(
                        "Missing Tax Withholding row after Vest Schedule \
                         for {} on {vest_date_str}",
                        current_grant_number
                    ));
                };

                let fmv_per_share = taxable_gain / released_qty;

                benefits.push(BenefitEntry {
                    security: current_symbol.clone(),
                    acquire_tx_date: vest_date,
                    acquire_settle_date: vest_date,
                    acquire_share_price: fmv_per_share,
                    acquire_shares: released_qty,
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: format!("RSU {}", current_grant_number),
                    sell_note: None,
                    filename: filename.to_string(),
                });
            }
            _ => {} // Skip Event, Tax Withholding (handled via look-ahead), etc.
        }

        i += 1;
    }

    Ok(benefits)
}

#[cfg(feature = "xlsx_read")]
pub fn parse_benefit_history_xlsx(
    data: Vec<u8>,
    filename: &str,
    date_range: Option<&DateRange>,
) -> Result<Vec<BenefitEntry>, SError> {
    let sheets = crate::peripheral::excel::read_xl_data_sheets(
        data,
        &["ESPP", "Restricted Stock"],
    )?;
    let mut benefits = Vec::new();

    if sheets.is_empty() {
        return Err(
            "No ESPP or Restricted Stock sheets found in BenefitHistory xlsx"
                .to_string(),
        );
    }

    if let Some(espp_range) = sheets.get("ESPP") {
        benefits.extend(parse_espp_benefits_from_sheet(espp_range, filename)?);
    }
    if let Some(rs_range) = sheets.get("Restricted Stock") {
        benefits.extend(parse_rsu_benefits_from_sheet(rs_range, filename)?);
    }

    if let Some(range) = date_range {
        benefits.retain(|b| range.contains(&b.acquire_tx_date));
    }

    Ok(benefits)
}

/// Filters benefits to only those whose acquire_tx_date falls within the
/// given date range.
pub fn filter_benefits_by_date(
    benefits: &mut Vec<BenefitEntry>,
    date_range: &DateRange,
) {
    benefits.retain(|b| date_range.contains(&b.acquire_tx_date));
}

// MARK: tests

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::{
        peripheral::broker::BrokerTx,
        portfolio::TxAction,
        testlib::{assert_big_struct_eq, assert_vec_eq, assert_vecr_eq},
        util::date::parse_standard_date,
    };

    use super::{
        new_account, parse_eso_data, parse_eso_entries, parse_espp_entry,
        parse_post_ms_2023_trade_confirmation,
        parse_pre_ms_2023_trade_confirmations, parse_rsu_entry, BenefitCommonData,
        BenefitEntry, EsoData, EsoGrantData,
    };

    fn s(_str: &str) -> String {
        _str.to_string()
    }

    fn date(date_str: &str) -> time::Date {
        parse_standard_date(date_str).unwrap()
    }

    #[test]
    fn test_parse_rsu_entry() {
        // lopdf-based output
        let pdf_text = " Release Summary

            Account Number 11223344
            Tax Payment Method Sell-to-cover
            Company Name (Symbol) Foo Inc.
            (FOO)
            Award Number R98765
            Award Date 05-08-2020
            Award Type RSU
            Plan 2014
            Release Date 10-20-2023
            Shares Released 1,234.0000
            Market Value Per Share $215.350000
            Award Price Per Share $0.000000
            Sale Price Per Share $213.773300

            Release Details

            Calculation of Gain
            Market Value $26,488.05
            Award Price ($0.00)
            Total Gain $26,488.05

            Stock Distribution
            Award Shares 123.0000
            Shares Sold (67.0000)
            Shares Issued 56.0000

            Registration: Morgan Stanley Smith Barney
            Calculation of Taxes
            Taxable Gain $ Rate % Amount $
            Canada-BC 26,488.05 53.500 14,171.10
            Total Tax $14,171.10

            Cash Distribution
            Total Sale Price $14,322.81
            Total Tax ($14,171.10)
            Fee ($4.13)
            Total Due Participant $147.58

            EMPLOYEE STOCK PLAN RELEASE CONFIRMATION
            Provided by Foo Inc.
            John Doe
            Employee ID: 1111
            ";

        let rsu_entry = parse_rsu_entry(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/myrsu.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            rsu_entry,
            BenefitEntry {
                security: "FOO".to_string(),
                acquire_tx_date: date("2023-10-20"),
                acquire_settle_date: date("2023-10-20"),
                acquire_share_price: dec!(215.35),
                acquire_shares: dec!(1234),
                sell_to_cover_tx_date: None,
                sell_to_cover_settle_date: None,
                sell_to_cover_price: Some(dec!(213.7733)),
                sell_to_cover_shares: Some(dec!(67)),
                sell_to_cover_fee: Some(dec!(4.13)),
                plan_note: s("RSU R98765"),
                sell_note: None,
                filename: s("myrsu.pdf"),
            },
        );

        // pdfjs-based output
        if !crate::util::node::node_available() {
            if std::env::var("ALLOW_NO_NODE").is_ok() {
                eprintln!("WARNING: node not available, skipping pdfjs RSU test");
                return;
            }
            panic!("node not available. Set ALLOW_NO_NODE=1 to skip pdfjs tests.");
        }
        let pdf_text = "Release Summary
Account Number 11223344
Tax Payment Method Sell-to-cover
Company Name (Symbol) FOO SYSTEMS, INC.
(FOO)
Award Number R98765
Award Date 05-08-2020
Award Type RSU
Plan 2014
Release Date 10-20-2023
Shares Released 1,234.0000
Market Value Per Share $215.350000
Award Price Per Share $0.000000
Sale Price Per Share $213.773300
Release Details
Calculation of Gain
Market Value $26,488.05
Award Price ($0.00)
Total Gain $26,488.05
Stock Distribution
Award Shares 123.0000
Shares Sold (67.0000)
Shares Issued 56.0000
Registration: Morgan Stanley Smith Barney
Calculation of Taxes
Taxable Gain $ Rate % Amount $
Canada-BC 26,488.05 53.50000 14,171.10
Total Tax $14,171.10
Cash Distribution
Total Sale Price $14,322.81
Total Tax ($14,171.10)
Fee ($4.13)
Total Due Participant $147.58
EMPLOYEE STOCK PLAN RELEASE CONFIRMATION
Provided by Foo Inc.
John Doe
1234 Main St
Halifax, NS CA B3D 8
Employee ID: 1111
This information was provided by your company to E*TRADE from Morgan Stanley, a registered trademark of Morgan Stanley Smith Barney LLC, Member SIPC.
Morgan Stanley and its affiliates do not guarantee the accuracy or completeness of the information provided by your company. If a sale is associated with these shares,
the trade confirmation provided by Morgan Stanley Smith Barney LLC is the accurate record of the sales transaction.
The data shown on this statement is based on your company's records. The company reserves the right to make corrections to this data. The release shown on this
statement is subject to the terms of the plan under which the release was made.
12/01/2025 03:35:23 PM ET Page 1 of1
";

        let rsu_entry = parse_rsu_entry(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/myrsu.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            rsu_entry,
            BenefitEntry {
                security: "FOO".to_string(),
                acquire_tx_date: date("2023-10-20"),
                acquire_settle_date: date("2023-10-20"),
                acquire_share_price: dec!(215.35),
                acquire_shares: dec!(1234),
                sell_to_cover_tx_date: None,
                sell_to_cover_settle_date: None,
                sell_to_cover_price: Some(dec!(213.7733)),
                sell_to_cover_shares: Some(dec!(67)),
                sell_to_cover_fee: Some(dec!(4.13)),
                plan_note: s("RSU R98765"),
                sell_note: None,
                filename: s("myrsu.pdf"),
            },
        );
    }

    // lopdf-based output
    const SAMPLE_ESO: &str = "
        Order Number 12345678
        Account Stock Plan (FOO) -0112
        Order Type Same-Day Sale
        Company Name (Symbol) FOO COMPANY,
        INC.(FOO)
        Shares Exercised 1002
        Shares Sold 1002
        Price Type Market
        Limit Price N/A
        Term Good for Day
         Gross Proceeds $12,345.67
        Total Price ($1,234.56)
        Commission ($4.95)
        Sec Fee ($0.11)
        Broker Assist Fee ($0.00)
        Disbursement Fee ($0.00)
        Taxes Withheld ($2,345.67)
        Net Proceeds $23,456.78
        
         Exercise Details
        
        Exercise Date: 10/20/2024 Exercise Type: Same-Day Sale Registration:
        Grant 1 Grant 2
        Grant Date 1/1/2012 2/2/2013
        Grant Number 1234 1235
        Grant Type Nonqual Nonqual
        Grant Price $3.33 $4.44
        Sale Price $1,001.00 $2,001.00
        Exercise Market Value $1,000.00 $2,000.00
        Shares Exercised 100 200
        Shares Sold 31 91
        Total Gain $7,234.12 $10,101.11
        Taxable Gain $7,234.12 $10,101.11
        Gross Proceeds $9,876.54 $15,000.23
        Total Price $1,234.00 $123.32
        Comission/Fee $10.00 $11.00
        EMPLOYEE STOCK PLAN EXERCISE CONFIRMATION
        NO BODY
        1234 MAIN ST
        HALIFAX, NS B3D 8
         Employee ID: 1111
";

    #[test]
    fn test_parse_eso_data() {
        let eso_data = parse_eso_data(SAMPLE_ESO).unwrap();
        assert_big_struct_eq(
            eso_data,
            EsoData {
                common_benefit_data: BenefitCommonData {
                    employee_id: s("1111"),
                    account_number: s("0112"),
                    symbol: s("FOO"),
                },
                exercise_type: s("Same-Day Sale"),
                exercise_date: date("2024-10-20"),
                shares_sold: dec!(1002),
                grants: vec![
                    EsoGrantData {
                        grant_number: 1234,
                        exercise_fmv: dec!(1000.00),
                        shares_exercised: dec!(100),
                        sale_price: dec!(1001.00),
                        fee: dec!(10.00),
                    },
                    EsoGrantData {
                        grant_number: 1235,
                        exercise_fmv: dec!(2000.00),
                        shares_exercised: dec!(200),
                        sale_price: dec!(2001.00),
                        fee: dec!(11.00),
                    },
                ],
            },
        );
    }

    #[test]
    fn test_parse_eso_entries() {
        // Sale prices must all be equal
        let fixed_eso_data = SAMPLE_ESO.replace(
            "Sale Price $1,001.00 $2,001.00",
            "Sale Price $1,001.00 $1,001.00",
        );

        let eso_entries = parse_eso_entries(
            &fixed_eso_data,
            &std::path::PathBuf::from("foo/bar/myeso.pdf"),
        )
        .unwrap();
        assert_vec_eq(
            eso_entries,
            vec![
                BenefitEntry {
                    security: s("FOO"),
                    acquire_tx_date: date("2024-10-20"),
                    acquire_settle_date: date("2024-10-20"),
                    acquire_share_price: dec!(1000.00),
                    acquire_shares: dec!(100),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: s("Option Grant 1234"),
                    sell_note: Some(s("Same-Day Sale")),
                    filename: s("myeso.pdf"),
                },
                BenefitEntry {
                    security: s("FOO"),
                    acquire_tx_date: date("2024-10-20"),
                    acquire_settle_date: date("2024-10-20"),
                    acquire_share_price: dec!(2000.00),
                    acquire_shares: dec!(200),
                    sell_to_cover_tx_date: Some(date("2024-10-20")),
                    sell_to_cover_settle_date: Some(date("2024-10-20")),
                    sell_to_cover_price: Some(dec!(1001.00)),
                    sell_to_cover_shares: Some(dec!(1002)),
                    sell_to_cover_fee: Some(dec!(21.00)),
                    plan_note: s("Option Grant 1235"),
                    sell_note: Some(s("Same-Day Sale")),
                    filename: s("myeso.pdf"),
                },
            ],
        );
    }

    #[test]
    fn test_parse_espp_entry() {
        // lopdf-based output
        let pdf_text = " Purchase Summary

        Account Number 11223344
        Company Name (Symbol) Foo Systems,
        INC.(FOO)
        Plan ESP2
        Grant Date 08-01-2022
        Purchase Begin Date 01-01-2023
        Purchase Date 10-20-2023
        Shares Purchased to Date in Current Offering
        Beginning Balance 0.0000
        Shares Purchased 123.0000
        Total shares Purchased for Offering 124.0000
        Shares Deposited in STREETNAME to
        ETRADE 124.0000

        Shares Sold to Cover Taxes 67.0000

        Purchase Details

        Contributions
        Foreign Contributions 1,000,000.00
        Average Exchange Rate $0.740000
        Previous Carry Forward $0.00
        Current Contributions $0.00
        Total Contributions $0.00*

        Total Price ($5,000.00)
        Carry Forward ($0.00)

        Calculation of Gain
        Total Value $1,000,000.00
        Total Price ($1,000,000.00)
        Taxable Gain $1,000,000.00
        Calculation of Shares Purchased
        Grant Date Market Value $10.990
        Purchase Value per Share $215.350000
        Purchase Price per Share
                (90.000% of $215.350000) $193.81500
        Total Price
                (Shares Purchased x Purchase Price) $1,000,000.00
        Sale Price for Shares Sold to Cover Taxes $213.773300

        Tax Assessment $1,840.84 Fees ($4.13)
        Adjusted Tax Assessment $1,000,000.00
        Amount in Excess of Tax Due $0.00

        Excess of Taxes Applied To

        Cash Due Participant

        Net Carry Forward $0.00

        EMPLOYEE STOCK PLAN PURCHASE CONFIRMATION
        Provided by Foo Inc.
        John Doe
        Employee ID: 1111
        ";

        let espp_entry = parse_espp_entry(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/myespp.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            espp_entry,
            BenefitEntry {
                security: "FOO".to_string(),
                acquire_tx_date: date("2023-10-20"),
                acquire_settle_date: date("2023-10-20"),
                acquire_share_price: dec!(215.35),
                acquire_shares: dec!(123),
                sell_to_cover_tx_date: None,
                sell_to_cover_settle_date: None,
                sell_to_cover_price: Some(dec!(213.7733)),
                sell_to_cover_shares: Some(dec!(67)),
                sell_to_cover_fee: Some(dec!(4.13)),
                plan_note: s("ESPP"),
                sell_note: None,
                filename: s("myespp.pdf"),
            },
        );

        // pdfjs-based output
        if !crate::util::node::node_available() {
            if std::env::var("ALLOW_NO_NODE").is_ok() {
                eprintln!("WARNING: node not available, skipping pdfjs ESPP test");
                return;
            }
            panic!("node not available. Set ALLOW_NO_NODE=1 to skip pdfjs tests.");
        }
        let pdf_text = "Purchase Summary
Account Number 11223344
Company Name (Symbol) Foo Systems,
INC.(FOO)
Plan ESP2
Grant Date 08-01-2022
Purchase Begin Date 01-01-2023
Purchase Date 10-20-2023
Shares Purchased to Date in Current Offering
Beginning Balance 0.0000
Shares Purchased 123.0000
Total shares Purchased for Offering 124.0000
Shares Deposited in STREETNAME to
Morgan Stanley Smith Barney
124.0000
Shares Sold to Cover Taxes 67.0000
Purchase Details
Contributions
Foreign Contributions 1,000,000.00
Average Exchange Rate $0.740000
Previous Carry Forward $0.00
Current Contributions $0.00
Total Contributions $0.00*
Total Price ($5,000.00)
Carry Forward ($0.00)
Calculation of Gain
Total Value $1,000,000.00
Total Price ($1,000,000.00)
Taxable Gain $1,000,000.00
Calculation of Shares Purchased
Grant Date Market Value $10.990
Purchase Value per Share $215.350000
Purchase Price per Share
(90.000% of $215.350000) $193.81500
Total Price
(Shares Purchased x Purchase Price) $1,000,000.00
Sale Price for Shares Sold to Cover Taxes $213.773300
Tax Assessment $1,840.84
Fees ($4.13)
Adjusted Tax Assessment $1,000,000.00
Amount in Excess of Tax Due $0.00
Excess of Taxes Applied To
Cash Due Participant
Net Carry Forward $0.00
EMPLOYEE STOCK PLAN PURCHASE CONFIRMATION
Provided by Foo Inc.
John Doe
1234 Main St
Halifax, NS CA B3D 8
Employee ID: 1111
This information was provided to E*TRADE Securities LLC, a subsidiary of Morgan Stanley, (\"E*TRADE\") by your company. E*TRADE does not guarantee the accuracy
or completeness of the information provided by your company. If a sale is associated with these shares, the trade confirmation provided by E*TRADE Securities LLC is
the only accurate record of the sale transaction.
The data shown on this statement is based on your company's records. The company reserves the right to make corrections to this data. The purchase shown on this
statement is subject to the terms of the plan under which the purchase was made.
12/01/2025 03:34:58 PM ET Page 1 of1
";

        let espp_entry = parse_espp_entry(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/myespp.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            espp_entry,
            BenefitEntry {
                security: "FOO".to_string(),
                acquire_tx_date: date("2023-10-20"),
                acquire_settle_date: date("2023-10-20"),
                acquire_share_price: dec!(215.35),
                acquire_shares: dec!(123),
                sell_to_cover_tx_date: None,
                sell_to_cover_settle_date: None,
                sell_to_cover_price: Some(dec!(213.7733)),
                sell_to_cover_shares: Some(dec!(67)),
                sell_to_cover_fee: Some(dec!(4.13)),
                plan_note: s("ESPP"),
                sell_note: None,
                filename: s("myespp.pdf"),
            },
        );

        // Test no sell-to-cover
        // lopdf-based output
        let pdf_text = " Purchase Summary

        Account Number 11223344
        Company Name (Symbol) Foo Systems,
        INC.(FOO)
        Plan ESP2
        Grant Date 08-01-2022
        Purchase Begin Date 01-01-2023
        Purchase Date 10-20-2023
        Shares Purchased to Date in Current Offering
        Beginning Balance 0.0000
        Shares Purchased 123.0000
        Total shares Purchased for Offering 124.0000
        Shares Deposited in STREETNAME to
        ETRADE 124.0000

        Total Price ($5,000.00)

        Calculation of Gain
        Total Value $1,000,000.00
        Total Price ($1,000,000.00)
        Taxable Gain $1,000,000.00
        Calculation of Shares Purchased
        Grant Date Market Value $10.990
        Purchase Value per Share $215.350000
        Purchase Price per Share
                (90.000% of $215.350000) $193.81500
        Total Price
                (Shares Purchased x Purchase Price) $1,000,000.00


        EMPLOYEE STOCK PLAN PURCHASE CONFIRMATION
        Employee ID: 1111
        ";

        let espp_entry = parse_espp_entry(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/myespp.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            espp_entry,
            BenefitEntry {
                security: "FOO".to_string(),
                acquire_tx_date: date("2023-10-20"),
                acquire_settle_date: date("2023-10-20"),
                acquire_share_price: dec!(215.35),
                acquire_shares: dec!(123),
                sell_to_cover_tx_date: None,
                sell_to_cover_settle_date: None,
                sell_to_cover_price: None,
                sell_to_cover_shares: None,
                sell_to_cover_fee: None,
                plan_note: s("ESPP"),
                sell_note: None,
                filename: s("myespp.pdf"),
            },
        );
    }

    #[test]
    fn test_parse_pre_ms_2023_trade_confirmations() {
        // lopdf-based output
        let pdf_text = "
            E*TRADE Securities LLC
            P.O. Box 484
            Jersey City, NJ 07303-0484

            DETACH HERE DETACH HERE

            Make checks payable to E*TRADE Securities LLC.
            Mail deposits to:
            E TRADE

            Please do not send cash Dollars Cents

            TOTAL DEPOSIT

            Account Name:
            John Doe

            E TRADE Securities LLC
            P.O. Box 484
            Jersey City, NJ 07303-0484
            021620230001 900361157028

            Account Number: XXXX-9876
            Use This Deposit Slip Acct: XXXX-9876

            Investment Account

            John Doe
            Employee ID: 1111
            TRADE CONFIRMATION

            Page 1 of 2

            TRADE
            DATE SETL
            DATE MKT /
            CPT SYMBOL /
            CUSIP BUY /
            SELL QUANTITY PRICE ACCT
            TYPE
            02/20/23 02/22/23 6 1 FOO SELL 6 $120.01 Stock Plan PRINCIPAL $720.06
            FOOSYSTEMS INC COM COMMISSION $20.05
            FEE $0.02
            NET AMOUNT $740.13

            02/20/23 02/22/23 6 1 FOO SELL 1 $120.011 Stock Plan PRINCIPAL $120.01
            FOOSYSTEMS INC COM FEE $0.01
            NET AMOUNT $120.02

            JOHN DOE
            1 BLAH DR
            VANCOUVER BCHOH OHO
            CANADA
            ";

        // (Note how the mkt and cpt here are joined. These values don't actually
        // get used, so this doesn't matter much).
        let pypdf_text = "
            E*TRADE Securities LLC
            P.O. Box484
            Jersey City, NJ07303-0484DETACH HERE DETACH HERE
            Make checks payable toE*TRADE Securities LLC.
            Mail deposits to:ETRADE
            Please donotsend cash Dollars Cents
            TOTAL DEPOSITAccount Name:
            JOHN DOE
            ETRADE Securities LLC
            P.O.Box484
            Jersey City, NJ07303-0484
            021620220001 900123456788Account Number: XXXX-9876
            UseThis Deposit Slip Acct: XXXX-9876Investment Account
            JOHN DOE
            1 BLAH DR
            VANCOUVER BCHOH OHO
            CANADATRADECONFIRMATIONPage 1of2
            TRADE
            DATESETL
            DATEMKT /
            CPTSYMBOL /
            CUSIPBUY /
            SELL QUANTITY PRICEACCT
            TYPE
            02/20/23 02/22/23 61 FOO SELL 6 $120.01 Stock Plan PRINCIPAL $720.06
            FOOSYSTEMS INC COM COMMISSION $20.05
            FEE $0.02
            NET AMOUNT $740.13
            02/20/23 02/22/23 61 FOO SELL 1 $120.011 Stock Plan PRINCIPAL $120.01
            FOOSYSTEMS INC COM FEE $0.01
            NET AMOUNT $120.02

            JOHN DOE
            1 BLAH DR
            VANCOUVER BCHOH OHO
            CANADA
        ";
        let exp_txs = vec![
            BrokerTx {
                security: s("FOO"),
                trade_date: date("2023-02-20"),
                settlement_date: date("2023-02-22"),
                trade_date_and_time: s("02/20/23"),
                settlement_date_and_time: s("02/22/23"),
                action: TxAction::Sell,
                amount_per_share: dec!(120.01),
                num_shares: dec!(6),
                commission: dec!(20.07),
                currency: crate::portfolio::Currency::usd(),
                memo: s(""),
                exchange_rate: None,
                affiliate: crate::portfolio::Affiliate::default(),
                row_num: 1,
                account: new_account(s("XXXX-9876")),
                sort_tiebreak: None,
                filename: Some(s("tconf.pdf")),
            },
            BrokerTx {
                security: s("FOO"),
                trade_date: date("2023-02-20"),
                settlement_date: date("2023-02-22"),
                trade_date_and_time: s("02/20/23"),
                settlement_date_and_time: s("02/22/23"),
                action: TxAction::Sell,
                amount_per_share: dec!(120.011),
                num_shares: dec!(1),
                commission: dec!(0.01),
                currency: crate::portfolio::Currency::usd(),
                memo: s(""),
                exchange_rate: None,
                affiliate: crate::portfolio::Affiliate::default(),
                row_num: 2,
                account: new_account(s("XXXX-9876")),
                sort_tiebreak: None,
                filename: Some(s("tconf.pdf")),
            },
        ];

        let txs = parse_pre_ms_2023_trade_confirmations(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/tconf.pdf"),
        )
        .unwrap();
        assert_vecr_eq(&txs, &exp_txs);

        let py_txs = parse_pre_ms_2023_trade_confirmations(
            pypdf_text,
            &std::path::PathBuf::from("foo/bar/tconf.pdf"),
        )
        .unwrap();
        assert_vecr_eq(&py_txs, &exp_txs);
    }

    #[test]
    fn test_parse_post_ms_2023_trade_confirmation() {
        // pypdf-based output
        let pdf_text = "
            Morgan Stanley Smith Barney LLC. Member SIPC. The transaction(s) may have been executed with Morgan Stanley & Co. LLC, an
            affiliate, which may receive compensation for any such services. E*TRADE is a business of Morgan Stanley.
            1 of 2Your Account Number: 123-XXX123-123
            Account Type - Cash
            John Doe
            E*TRADE from Morgan Stanley
            P.O. BOX 484
            JERSEY CITY, NJ 07303-0484
            (800)-387-2331
            This transaction is confirmed in accordance with the information provided on the Conditions and Disclosures page.
            Trade Date Settlement Date Quantity Price Settlement Amount
            11/01/2023 11/03/2023 123 200.01
            Transaction Type: Sold Short
            Description: FOOSYSTEMS INC
            Symbol / CUSIP / ISIN: FOO / 123456789 / US0123456789Principal $24,601.23
            Commission $3.91
            Supplemental
            Transaction Fee $0.21
            Net Amount $24,605.35
            Unsolicited trade
            Morgan Stanley Smith Barney LLC acted as agent.
            ";

        let tx = parse_post_ms_2023_trade_confirmation(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/tconf.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            tx,
            BrokerTx {
                security: s("FOO"),
                trade_date: date("2023-11-01"),
                settlement_date: date("2023-11-03"),
                trade_date_and_time: s("11/01/2023"),
                settlement_date_and_time: s("11/03/2023"),
                action: TxAction::Sell,
                amount_per_share: dec!(200.01),
                num_shares: dec!(123),
                commission: dec!(4.12),
                currency: crate::portfolio::Currency::usd(),
                memo: s(""),
                exchange_rate: None,
                affiliate: crate::portfolio::Affiliate::default(),
                row_num: 1,
                account: new_account(s("123-XXX123-123")),
                sort_tiebreak: None,
                filename: Some(s("tconf.pdf")),
            },
        );

        // Alt (2025 Q4 and later?) format
        // pypdf-based output (does not work with lopdf)
        let pdf_text = "
            This transaction is confirmed in accordance with the information provided on the Conditions and Disclosures page.Your Account Number:
            E*TRADE from Morgan Stanley
            Trade Date Settlement Date Quantity Price Settlement Amount
            Transaction Type: Sold Short
            Description: FOOSYSTEMS INCNet Amount $24,601.23 
            JOHN DOE
            1234 STREET AVE.
            VANCOUVER BRITISH COLUMBIA H0H 0H0
            CANADA123-XXX123-123
            Account Type - Cash
            P.O. BOX 484
            JERSEY CITY, NJ 07303-0484
            (800)-387-2331
            Morgan Stanley Smith Barney LLC. Member SIPC. The transaction(s) may have been executed with Morgan Stanley & Co. LLC, an
            affiliate, which may receive compensation for any such services. E*TRADE is a business of Morgan Stanley.
            1 of 211/21/2025 11/24/2025 123 200.01
            Symbol / CUSIP / ISIN: FOO / 123456789 / US0123456789Principal $24,601.23
            Commission $3.91
            Unsolicited trade
            Sell short exempt
            Morgan Stanley Smith Barney LLC acted as agent.

            2 of 2
        ";

        let tx = parse_post_ms_2023_trade_confirmation(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/tconf.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            tx,
            BrokerTx {
                security: s("FOO"),
                trade_date: date("2025-11-21"),
                settlement_date: date("2025-11-24"),
                trade_date_and_time: s("11/21/2025"),
                settlement_date_and_time: s("11/24/2025"),
                action: TxAction::Sell,
                amount_per_share: dec!(200.01),
                num_shares: dec!(123),
                commission: dec!(3.91),
                currency: crate::portfolio::Currency::usd(),
                memo: s(""),
                exchange_rate: None,
                affiliate: crate::portfolio::Affiliate::default(),
                row_num: 1,
                account: new_account(s("123-XXX123-123")),
                sort_tiebreak: None,
                filename: Some(s("tconf.pdf")),
            },
        );

        // pdfjs-based output
        if !crate::util::node::node_available() {
            if std::env::var("ALLOW_NO_NODE").is_ok() {
                eprintln!(
                    "WARNING: node not available, skipping pdfjs trade confirmation test"
                );
                return;
            }
            panic!("node not available. Set ALLOW_NO_NODE=1 to skip pdfjs tests.");
        }
        let pdf_text = "Morgan Stanley Smith Barney LLC. Member SIPC. The transaction(s) may have been executed with Morgan Stanley & Co. LLC, an
affiliate, which may receive compensation for any such services. E*TRADE is a business of Morgan Stanley.
1 of 2
Your Account Number: 123-XXX123-123
Account Type - Cash
JOHN DOE
1234 STREET AVE.
VANCOUVER BRITISH COLUMBIA H0H 0H0
CANADA
E*TRADE from Morgan Stanley
P.O. BOX 484
JERSEY CITY, NJ 07303-0484
(800)-387-2331
This transaction is confirmed in accordance with the information provided on the Conditions and Disclosures page.
Trade Date Settlement Date Quantity Price Settlement Amount
11/01/2023 11/03/2023 123 200.01
Transaction Type: Sold Short
Description: FOOSYSTEMS INC
Symbol / CUSIP / ISIN: FOO / 123456789 / US0123456789
Principal $24,601.23
Commission $3.91
Supplemental
Transaction Fee $0.21
Net Amount $24,605.35
Unsolicited trade
Sell short exempt
Morgan Stanley Smith Barney LLC acted as agent.
2 of 2
";

        let tx = parse_post_ms_2023_trade_confirmation(
            pdf_text,
            &std::path::PathBuf::from("foo/bar/tconf.pdf"),
        )
        .unwrap();

        assert_big_struct_eq(
            tx,
            BrokerTx {
                security: s("FOO"),
                trade_date: date("2023-11-01"),
                settlement_date: date("2023-11-03"),
                trade_date_and_time: s("11/01/2023"),
                settlement_date_and_time: s("11/03/2023"),
                action: TxAction::Sell,
                amount_per_share: dec!(200.01),
                num_shares: dec!(123),
                commission: dec!(4.12),
                currency: crate::portfolio::Currency::usd(),
                memo: s(""),
                exchange_rate: None,
                affiliate: crate::portfolio::Affiliate::default(),
                row_num: 1,
                account: new_account(s("123-XXX123-123")),
                sort_tiebreak: None,
                filename: Some(s("tconf.pdf")),
            },
        );
    }

    // MARK: BenefitHistory xlsx tests

    #[cfg(all(feature = "xlsx_read", feature = "xlsx_write"))]
    mod benefit_history_tests {
        use rust_decimal_macros::dec;
        use rust_xlsxwriter::Workbook;

        use crate::{
            testlib::assert_big_struct_eq, util::date::parse_standard_date,
        };

        use super::super::{
            filter_benefits_by_date, parse_benefit_history_xlsx,
            parse_etrade_dmy_date, BenefitEntry,
        };
        use crate::util::date::DateRange;

        fn date(date_str: &str) -> time::Date {
            parse_standard_date(date_str).unwrap()
        }

        fn make_xlsx_bytes(build: impl FnOnce(&mut Workbook)) -> Vec<u8> {
            let mut wb = Workbook::new();
            build(&mut wb);
            wb.save_to_buffer().unwrap()
        }

        #[test]
        fn test_parse_dmy_date() {
            assert_eq!(
                parse_etrade_dmy_date("15-FEB-2017").unwrap(),
                date("2017-02-15")
            );
            assert_eq!(
                parse_etrade_dmy_date("01-JAN-2024").unwrap(),
                date("2024-01-01")
            );
            assert_eq!(
                parse_etrade_dmy_date("31-DEC-2020").unwrap(),
                date("2020-12-31")
            );
            assert!(parse_etrade_dmy_date("bad-date").is_err());
            assert!(parse_etrade_dmy_date("01-XYZ-2020").is_err());
        }

        fn build_espp_sheet(wb: &mut Workbook) {
            let sheet = wb.add_worksheet().set_name("ESPP").unwrap();
            // Header
            let headers = [
                "Record Type",
                "Symbol",
                "Purchase Date",
                "Purchase Price",
                "Purchased Qty.",
                "Tax Collection Shares",
                "Net Shares",
                "Sellable Qty.",
                "Est. Market Value",
                "Grant Date",
                "Discount Percent",
                "Grant Date FMV",
                "Purchase Date FMV",
                "Qualified Plan?",
                "Contribution Source",
                "Pending Sale Qty.",
                "Blocked Qty.",
                "Transferable Date",
                "First Sellable Date",
                "Date",
                "Event Type",
                "Qty",
            ];
            for (c, h) in headers.iter().enumerate() {
                sheet.write(0, c as u16, *h).unwrap();
            }
            // Purchase row: no sell-to-cover
            sheet.write(1, 0, "Purchase").unwrap();
            sheet.write(1, 1, "FOO").unwrap();
            sheet.write(1, 2, "15-FEB-2017").unwrap();
            sheet.write(1, 3, 50.28).unwrap();
            sheet.write(1, 4, 32.0).unwrap();
            sheet.write(1, 5, 0.0).unwrap();
            sheet.write(1, 6, 32.0).unwrap();
            sheet.write(1, 12, "$100.67").unwrap();
            // Event row (should be skipped)
            sheet.write(2, 0, "Event").unwrap();
            sheet.write(2, 19, "02/15/2017").unwrap();
            sheet.write(2, 20, "PURCHASE").unwrap();
            sheet.write(2, 21, 32.0).unwrap();
            // Purchase row: with sell-to-cover
            sheet.write(3, 0, "Purchase").unwrap();
            sheet.write(3, 1, "FOO").unwrap();
            sheet.write(3, 2, "15-FEB-2022").unwrap();
            sheet.write(3, 3, 45.69).unwrap();
            sheet.write(3, 4, 57.0).unwrap();
            sheet.write(3, 5, 21.0).unwrap();
            sheet.write(3, 6, 36.0).unwrap();
            sheet.write(3, 12, "$129.94").unwrap();
        }

        #[test]
        fn test_parse_espp_benefits() {
            let data = make_xlsx_bytes(build_espp_sheet);
            let benefits =
                parse_benefit_history_xlsx(data, "test.xlsx", None).unwrap();
            assert_eq!(benefits.len(), 2);

            assert_big_struct_eq(
                benefits[0].clone(),
                BenefitEntry {
                    security: "FOO".to_string(),
                    acquire_tx_date: date("2017-02-15"),
                    acquire_settle_date: date("2017-02-15"),
                    acquire_share_price: dec!(100.67),
                    acquire_shares: dec!(32),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: "ESPP".to_string(),
                    sell_note: None,
                    filename: "test.xlsx".to_string(),
                },
            );

            assert_big_struct_eq(
                benefits[1].clone(),
                BenefitEntry {
                    security: "FOO".to_string(),
                    acquire_tx_date: date("2022-02-15"),
                    acquire_settle_date: date("2022-02-15"),
                    acquire_share_price: dec!(129.94),
                    acquire_shares: dec!(57),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: Some(dec!(21)),
                    sell_to_cover_fee: None,
                    plan_note: "ESPP".to_string(),
                    sell_note: None,
                    filename: "test.xlsx".to_string(),
                },
            );
        }

        fn build_rsu_sheet(wb: &mut Workbook) {
            let sheet = wb.add_worksheet().set_name("Restricted Stock").unwrap();
            // Header — only columns we actually read need real names;
            // the rest are placeholders.
            let headers = [
                "Record Type",     // 0
                "Symbol",          // 1
                "Grant Date",      // 2
                "Settlement Type", // 3
                "Granted Qty.",    // 4
                "Withheld Qty.",   // 5
                "Vested Qty.",     // 6
                "col7",
                "col8",
                "col9",         // 7-9
                "Grant Number", // 10
                "col11",
                "col12",
                "col13",
                "col14",
                "col15",
                "col16",
                "col17",
                "col18",
                "col19",
                "col20",
                "Date",           // 21
                "Event Type",     // 22
                "Qty. or Amount", // 23
                "Vest Period",    // 24
                "Vest Date",      // 25
                "col26",
                "col27",
                "col28",
                "col29",
                "col30",
                "col31",
                "Vested Qty.",        // 32 (duplicate name — last wins)
                "Released Qty",       // 33
                "Released Amount",    // 34
                "Sellable Qty.",      // 35
                "Blocked Qty.",       // 36
                "Total Taxes Paid",   // 37
                "Tax Description",    // 38
                "Taxable Gain",       // 39
                "Effective Tax Rate", // 40
                "Withholding Amount", // 41
            ];
            for (c, h) in headers.iter().enumerate() {
                sheet.write(0, c as u16, *h).unwrap();
            }
            // Grant row
            sheet.write(1, 0, "Grant").unwrap();
            sheet.write(1, 1, "BAR").unwrap();
            sheet.write(1, 2, "08-MAY-2020").unwrap();
            sheet.write(1, 10, "R12345").unwrap();
            // Event row (skipped)
            sheet.write(2, 0, "Event").unwrap();
            sheet.write(2, 10, "R12345").unwrap();
            sheet.write(2, 21, "02/22/2021").unwrap();
            sheet.write(2, 22, "Shares sold").unwrap();
            sheet.write(2, 23, 50.0).unwrap();
            // Vest Schedule with Released Qty > 0
            sheet.write(3, 0, "Vest Schedule").unwrap();
            sheet.write(3, 10, "R12345").unwrap();
            sheet.write(3, 24, 1.0).unwrap(); // Vest Period
            sheet.write(3, 25, "02/20/2021").unwrap(); // Vest Date
            sheet.write(3, 33, 100.0).unwrap(); // Released Qty
            sheet.write(3, 37, 5000.0).unwrap(); // Total Taxes Paid
                                                 // Tax Withholding for vest period 1
            sheet.write(4, 0, "Tax Withholding").unwrap();
            sheet.write(4, 10, "R12345").unwrap();
            sheet.write(4, 24, 1.0).unwrap();
            sheet.write(4, 39, 3000.0).unwrap(); // Taxable Gain
                                                 // Vest Schedule with Released Qty = 0 (future vest, skipped)
            sheet.write(5, 0, "Vest Schedule").unwrap();
            sheet.write(5, 10, "R12345").unwrap();
            sheet.write(5, 24, 2.0).unwrap();
            sheet.write(5, 25, "05/20/2026").unwrap();
            sheet.write(5, 33, 0.0).unwrap();
            // Second vest with data
            sheet.write(6, 0, "Vest Schedule").unwrap();
            sheet.write(6, 10, "R12345").unwrap();
            sheet.write(6, 24, 3.0).unwrap();
            sheet.write(6, 25, "08/20/2021").unwrap();
            sheet.write(6, 33, 200.0).unwrap();
            sheet.write(6, 37, 10000.0).unwrap();
            // Tax Withholding for vest period 3
            sheet.write(7, 0, "Tax Withholding").unwrap();
            sheet.write(7, 10, "R12345").unwrap();
            sheet.write(7, 24, 3.0).unwrap();
            sheet.write(7, 39, 8000.0).unwrap(); // FMV = 8000/200 = 40
        }

        #[test]
        fn test_parse_rsu_benefits() {
            let data = make_xlsx_bytes(build_rsu_sheet);
            let benefits =
                parse_benefit_history_xlsx(data, "test.xlsx", None).unwrap();
            assert_eq!(benefits.len(), 2);

            assert_big_struct_eq(
                benefits[0].clone(),
                BenefitEntry {
                    security: "BAR".to_string(),
                    acquire_tx_date: date("2021-02-20"),
                    acquire_settle_date: date("2021-02-20"),
                    acquire_share_price: dec!(30), // 3000 / 100
                    acquire_shares: dec!(100),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: "RSU R12345".to_string(),
                    sell_note: None,
                    filename: "test.xlsx".to_string(),
                },
            );

            assert_big_struct_eq(
                benefits[1].clone(),
                BenefitEntry {
                    security: "BAR".to_string(),
                    acquire_tx_date: date("2021-08-20"),
                    acquire_settle_date: date("2021-08-20"),
                    acquire_share_price: dec!(40), // 8000 / 200
                    acquire_shares: dec!(200),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: "RSU R12345".to_string(),
                    sell_note: None,
                    filename: "test.xlsx".to_string(),
                },
            );
        }

        #[test]
        fn test_parse_combined_espp_and_rsu() {
            let data = make_xlsx_bytes(|wb| {
                build_espp_sheet(wb);
                build_rsu_sheet(wb);
            });
            let benefits =
                parse_benefit_history_xlsx(data, "combined.xlsx", None).unwrap();
            // 2 ESPP + 2 RSU
            assert_eq!(benefits.len(), 4);
            assert_eq!(benefits[0].plan_note, "ESPP");
            assert_eq!(benefits[1].plan_note, "ESPP");
            assert_eq!(benefits[2].plan_note, "RSU R12345");
            assert_eq!(benefits[3].plan_note, "RSU R12345");
        }

        #[test]
        fn test_parse_espp_with_date_range_filter() {
            let data = make_xlsx_bytes(build_espp_sheet);
            let range = DateRange::for_year(2022);
            let benefits =
                parse_benefit_history_xlsx(data, "test.xlsx", Some(&range)).unwrap();
            // Only the 2022 ESPP entry should survive
            assert_eq!(benefits.len(), 1);
            assert_eq!(benefits[0].acquire_tx_date, date("2022-02-15"));
        }

        #[test]
        fn test_parse_rsu_with_date_range_filter() {
            let data = make_xlsx_bytes(build_rsu_sheet);
            // Both RSU vests are in 2021; filtering for 2022 should return none
            let range = DateRange::for_year(2022);
            let benefits =
                parse_benefit_history_xlsx(data.clone(), "test.xlsx", Some(&range))
                    .unwrap();
            assert_eq!(benefits.len(), 0);

            // Filtering for 2021 should return both
            let range = DateRange::for_year(2021);
            let benefits =
                parse_benefit_history_xlsx(data, "test.xlsx", Some(&range)).unwrap();
            assert_eq!(benefits.len(), 2);
        }

        #[test]
        fn test_filter_benefits_by_date() {
            let mut benefits = vec![
                BenefitEntry {
                    security: "FOO".to_string(),
                    acquire_tx_date: date("2021-03-15"),
                    acquire_settle_date: date("2021-03-15"),
                    acquire_share_price: dec!(50),
                    acquire_shares: dec!(10),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: "ESPP".to_string(),
                    sell_note: None,
                    filename: "test".to_string(),
                },
                BenefitEntry {
                    security: "FOO".to_string(),
                    acquire_tx_date: date("2022-06-15"),
                    acquire_settle_date: date("2022-06-15"),
                    acquire_share_price: dec!(60),
                    acquire_shares: dec!(20),
                    sell_to_cover_tx_date: None,
                    sell_to_cover_settle_date: None,
                    sell_to_cover_price: None,
                    sell_to_cover_shares: None,
                    sell_to_cover_fee: None,
                    plan_note: "ESPP".to_string(),
                    sell_note: None,
                    filename: "test".to_string(),
                },
            ];
            let range = DateRange::for_year(2022);
            filter_benefits_by_date(&mut benefits, &range);
            assert_eq!(benefits.len(), 1);
            assert_eq!(benefits[0].acquire_tx_date, date("2022-06-15"));
        }

        #[test]
        fn test_parse_empty_xlsx_errors() {
            let data = make_xlsx_bytes(|wb| {
                wb.add_worksheet().set_name("OtherSheet").unwrap();
            });
            assert!(parse_benefit_history_xlsx(data, "empty.xlsx", None).is_err());
        }
    }
}
