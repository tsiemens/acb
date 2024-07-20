use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use regex::{Regex, RegexBuilder};
use rust_decimal::Decimal;
use time::Date;

use crate::{
    app::outfmt::model::AcbWriter,
    peripheral::pdf,
    portfolio::render::RenderTable,
    util::{
        basic::SError, date::parse_month, decimal::dollar_precision_str,
        rw::WriteHandle,
    },
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Fmv {
    // This is a long security descriptor, not just the symbol, and may include
    pub security_desc: String,
    // Percentage of the portfolio, based on FMV
    pub allocation: Decimal,
    // In CAD
    pub fmv: Decimal,
}

mod sm {
    use regex::Regex;
    use rust_decimal::Decimal;

    use lazy_static::lazy_static;

    use crate::util::basic::SError;

    use super::Fmv;

    // Each row starts with this bullet/square, which is used to show the chart
    // color on the statement.
    const SEC_SEPARATOR: &str = "■";

    lazy_static! {
        static ref SEC_FIRST_ROW_RE: Regex =
            Regex::new(r"^\s*■\s*(\S.*)\s*$").unwrap();

        // Always ends with something like "80.0 800,000.0"
        static ref SEC_DATA_RE: Regex =
            Regex::new(r"^\s*(?P<desc>\S(.*\S)?)\s+(?P<alloc>\d[0-9\.]+)\s+(?P<fmv>\d[0-9,\.]*)\s*$")
            .unwrap();

        // Looks for a row like:
        // 100.0 1,000,000.0
        static ref TOTAL_ROW_RE: Regex =
            Regex::new(r"^\s*100.00?\s+(\d[0-9,\.]+)\s*$").unwrap();
    }

    enum State {
        LookingForHeader,
        LookingForFirstSecurityStart,
        GatheringSecurities,
    }

    pub struct FmvParseSm {
        pub fmvs: Vec<Fmv>,
        pub total_fmv: Decimal,

        state: State,
        // Working parsed values
        security_desc: String,
        // sec_allocation: Option<Decimal>,
        // sec_fmv: Option<Decimal>,
    }

    impl FmvParseSm {
        pub fn new() -> Self {
            FmvParseSm {
                fmvs: Vec::new(),
                total_fmv: Decimal::ZERO,
                state: State::LookingForHeader,
                security_desc: String::new(),
                // sec_allocation: None,
                // sec_fmv: None,
            }
        }

        pub fn parse_page(&mut self, page: &str) -> Result<(), SError> {
            for line in page.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                match self.state {
                    State::LookingForHeader => {
                        if line.contains("ALLOCATION") {
                            self.state = State::LookingForFirstSecurityStart;
                        }
                    }
                    State::LookingForFirstSecurityStart => {
                        if line.contains(SEC_SEPARATOR) {
                            self.state = State::GatheringSecurities;
                            self.gather_security_line(line)?;
                        } else if TOTAL_ROW_RE.is_match(line) {
                            // Done. No securities found.
                            self.gather_total_line(line)?;
                            return Ok(());
                        }
                    }
                    State::GatheringSecurities => {
                        if TOTAL_ROW_RE.is_match(line) {
                            // Maybe done. Try to terminate last security.
                            if self.finalize_security_fmv().is_err() {
                                // It is possible that this line is part of this
                                // security (thus why it failed). This can be when
                                // we have a single security, in which case, we can
                                // have two lines with 100%.
                                // Try to consider this as part of the last security.
                                self.gather_security_line(line)?;
                            } else {
                                // This was (most likely) the actual total row,
                                // so we are done.
                                self.gather_total_line(line)?;
                                return Ok(());
                            }
                        } else {
                            self.gather_security_line(line)?;
                        }
                    }
                }
            }

            Err("No header or allocation total line found".to_string())
        }

        /// The caller guarantees that we are on a line that is part of some
        /// security allocation table row. Collect this data. If we are starting
        /// a new security, log the old one and start a new one.
        fn gather_security_line(&mut self, line: &str) -> Result<(), SError> {
            // We are guaranteed not to be on the total row here.
            if let Some(m) = SEC_FIRST_ROW_RE.captures(line) {
                if !self.security_desc.is_empty() {
                    self.finalize_security_fmv()?;
                }
                self.security_desc = m.get(1).unwrap().as_str().to_string();
            } else if !line.trim().is_empty() {
                self.security_desc += format!(" {}", line.trim()).as_str();
            }

            Ok(())
        }

        // Assumes caller has already matched TOTAL_ROW_RE
        fn gather_total_line(&mut self, line: &str) -> Result<(), SError> {
            let m = TOTAL_ROW_RE.captures(line).unwrap();
            let fmv_m = m.get(1).unwrap();

            let fmv = crate::util::decimal::parse_large_decimal(fmv_m.as_str())
                .map_err(|e| {
                    format!(
                        "Unable to parse FMV from \"{}\": {}",
                        fmv_m.as_str(),
                        e.to_string()
                    )
                })?;

            self.total_fmv = fmv;
            Ok(())
        }

        /// The caller has determined that we're done with a security row
        /// in the allocation table. Parse out the data from security_desc and
        /// save it as an Fmv.
        fn finalize_security_fmv(&mut self) -> Result<(), SError> {
            let fmv = FmvParseSm::security_text_to_fmv(&self.security_desc)?;
            self.fmvs.push(fmv);
            Ok(())
        }

        pub fn security_text_to_fmv(security_text: &str) -> Result<Fmv, SError> {
            if let Some(m) = SEC_DATA_RE.captures(security_text) {
                let security_desc = m.name("desc").unwrap().as_str().to_string();
                let alloc_m = m.name("alloc").unwrap();
                let fmv_m = m.name("fmv").unwrap();

                let allocation =
                    Decimal::from_str_exact(alloc_m.as_str()).map_err(|e| {
                        format!(
                            "Unable to parse allocation from \"{}\": {}",
                            alloc_m.as_str(),
                            e.to_string()
                        )
                    })?;
                let fmv = crate::util::decimal::parse_large_decimal(fmv_m.as_str())
                    .map_err(|e| {
                        format!(
                            "Unable to parse FMV from \"{}\": {}",
                            fmv_m.as_str(),
                            e.to_string()
                        )
                    })?;

                Ok(Fmv {
                    security_desc,
                    allocation,
                    fmv,
                })
            } else {
                Err(format!(
                    "Unable to parse allocation and FMV from \"{}\"",
                    security_text
                ))
            }
        }
    }
}

/// In a page where we already know there is the allocation table, parses out
/// each security and its respective allocation. Returns these and the total.
///
/// A sample table (approximate render) would be like:
/// Securities Owned
/// Combined in (CAD)
///                      ALLOCATION (%)² | MARKET VALUE ($)³
/// +------------------------------------+------------------
/// | ■ SEC A DESCRIPTION          80.0  | 800,000.0
/// |   DESCRIPTION MAY CONTINUE         |
/// +------------------------------------+------------------
/// | ■ Other                      20.0  | 200,000.0
/// +------------------------------------+------------------
///                                100.0 | 1,000,000.0
fn parse_fmvs_from_page(page: &str) -> Result<(Vec<Fmv>, Decimal), SError> {
    let mut sm = sm::FmvParseSm::new();
    sm.parse_page(page)?;
    Ok((sm.fmvs, sm.total_fmv))
}

#[derive(PartialEq, Eq, Debug)]
pub struct StatementFmvs {
    pub month_date: Date,
    pub fmvs: Vec<Fmv>,
    pub total: Decimal,
}

/// Parses statement pdf text, and finds the FMV for each security
/// as well as the month the statement is for.
///
/// Usage:
/// parse_statement_text(my_vec_of_string.iter())
/// parse_statement_text(my_vec_of_rc_string.iter().cloned())
pub fn parse_statement_text<'a, I, T>(pages: I) -> Result<StatementFmvs, SError>
where
    I: Iterator<Item = T>,
    T: std::borrow::Borrow<String> + 'a,
{
    let mut month_date: Option<Date> = None;

    let current_month_re = RegexBuilder::new(
        r"\bCurrent month:\s+(?P<month>\S+) (?P<day>\d+), (?P<year>\d+)",
    )
    .case_insensitive(true)
    .build()
    .unwrap();

    let fmv_page_marker =
        Regex::new(r"Securities\s+Owned\s+Combined\s+in\s+\(CAD\)").unwrap();

    for page in pages {
        if month_date.is_none() {
            if let Some(m) = current_month_re.captures(page.borrow()) {
                if let Ok(month) = parse_month(m.name("month").unwrap().as_str()) {
                    let year = m
                        .name("year")
                        .unwrap()
                        .as_str()
                        .parse::<i32>()
                        .map_err(|e| e.to_string())?;
                    let day = m
                        .name("day")
                        .unwrap()
                        .as_str()
                        .parse::<u8>()
                        .map_err(|e| e.to_string())?;
                    month_date = Some(
                        Date::from_calendar_date(year, month, day)
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
        }

        if !fmv_page_marker.is_match(page.borrow()) {
            continue;
        }

        let (fmvs, total) = parse_fmvs_from_page(page.borrow())?;
        let some_month = month_date.ok_or("Could not find month")?;
        return Ok(StatementFmvs {
            month_date: some_month,
            fmvs,
            total,
        });
    }

    Err("Did not find FMVs in statement".to_string())
}

/// Security descriptions can get very long, and this makes the table output
/// all but useless (it's already hard as it is, because it renders a column per
/// security, which can itself get very wide).
/// Attempts to generate a reasonable abbreviation for the security.
fn sec_desc_abbrev(sec_desc: &str) -> String {
    // Many, if not all, descriptions will terminate in "(CODE)", which makes for a
    // good abbreviation. Try to find that.
    let candidate_1_re = Regex::new(r"\((\S{1,8})\)\s*$").unwrap();
    if let Some(m) = candidate_1_re.captures(sec_desc) {
        m.get(1).unwrap().as_str().to_string()
    } else {
        let useable_part_re = Regex::new(r"^[a-zA-Z\d]+$").unwrap();
        let parts: Vec<String> = sec_desc
            .split(" ")
            .map(|p| p.trim())
            .filter(|p| useable_part_re.is_match(p))
            .map(|p| p.to_string())
            .collect();

        let joined_parts: String = parts.join("");
        if joined_parts.len() <= 8 {
            joined_parts
        } else if parts.len() <= 8 {
            parts
                .into_iter()
                .map(|p| p.get(..1).unwrap().to_string())
                .collect::<Vec<String>>()
                .join("")
        } else {
            joined_parts.get(..joined_parts.len().min(8)).unwrap().to_string()
        }
    }
}

/// Used to make unique security abbreviations from the existing abbreviation,
/// in cases where there are collisions.
fn abbrev_alt(sec_abbrev: &str, alt: u32) -> String {
    format!("{sec_abbrev}_{alt}")
}

fn parse_statement(
    file_path: &PathBuf,
    parallel_pages: bool,
) -> Result<StatementFmvs, SError> {
    tracing::info!("Parsing {}...", file_path.to_string_lossy());
    let doc = lopdf::Document::load(&file_path).map_err(|e| {
        format!("Error loading {}: {}", file_path.to_string_lossy(), e)
    })?;
    // We expect out date and FMV to be on pages 1 and 7. Allow some wiggle
    // room and also check 6 and 8 before falling back to the rest.
    let optimal_page_groups = vec![vec![1, 7], vec![6, 8]];
    let page_groups = pdf::LazyPageTextVec::safe_page_chunks_with_remainder(
        &doc,
        &optimal_page_groups,
    );
    let mut lazy_pages =
        pdf::LazyPageTextVec::new(std::sync::Arc::new(doc), parallel_pages);
    let page_iter = lazy_pages.optimized_iter(page_groups);

    let statement_fmvs = parse_statement_text(page_iter.map(|(_, txt)| txt))
        .map_err(|e| format!("Error in {}: {}", file_path.to_string_lossy(), e))?;
    Ok(statement_fmvs)
}

/// Renders a table like so for the statements:
/// Month | Total FMV (CAD) | SEC1 | SEC2 | ...
/// Jan
fn render_table_for_statements(statements: &Vec<StatementFmvs>) -> RenderTable {
    let mut table = RenderTable::default();

    let mut securities = std::collections::HashSet::<String>::new();
    let mut all_fmvs = Vec::<Fmv>::new();

    for st in statements {
        for fmv in &st.fmvs {
            securities.insert(fmv.security_desc.clone());
            all_fmvs.push(fmv.clone());
        }
    }

    let mut sorted_securities = Vec::from_iter(securities.into_iter());
    sorted_securities.sort();

    let mut sec_abbrevs = HashMap::<String, &String>::new();
    for sec_desc in &sorted_securities {
        let abbrev = sec_desc_abbrev(sec_desc);
        if sec_abbrevs.contains_key(&abbrev) {
            let mut alt_n = 2;
            let mut alt = abbrev_alt(abbrev.as_str(), alt_n);
            while sec_abbrevs.contains_key(&alt) {
                alt_n += 1;
                alt = abbrev_alt(abbrev.as_str(), alt_n);
            }
            sec_abbrevs.insert(alt, sec_desc);
        } else {
            sec_abbrevs.insert(abbrev, sec_desc);
        }
    }

    let mut sorted_sec_abbrevs: Vec<&String> =
        Vec::from_iter(sec_abbrevs.iter().map(|(a, _)| a));
    sorted_sec_abbrevs.sort();

    for sec_ab in &sorted_sec_abbrevs {
        let sec_desc = sec_abbrevs.get(*sec_ab).unwrap();
        table.notes.push(format!("{sec_ab} = {sec_desc}"));
    }

    let s = String::from;

    table.header.append(&mut vec![s("Month"), s("Total FMV (CAD)")]);
    table
        .header
        .append(&mut sorted_sec_abbrevs.iter().map(|s| (**s).clone()).collect());

    for st in statements {
        let sec_desc_to_fmv = HashMap::<&String, &Fmv>::from_iter(
            st.fmvs.iter().map(|f| (&f.security_desc, f)),
        );
        let mut row = vec![
            crate::util::date::to_pretty_string(&st.month_date),
            dollar_precision_str(&st.total),
        ];
        for sec_ab in &sorted_sec_abbrevs {
            let sec_desc = sec_abbrevs.get(*sec_ab).unwrap();
            if let Some(fmv) = sec_desc_to_fmv.get(sec_desc) {
                row.push(dollar_precision_str(&fmv.fmv))
            } else {
                row.push(s("-"));
            }
        }
        table.rows.push(row);
    }

    table
}

/// Utility to parse though Questrade statements and fetch the fair-market-values
/// (FMV)
///
/// Produces a CSV (to stdout) with the total FMV of each month, and the FMV of
/// each held security.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
struct Args {
    /// Questrade statement PDFs
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    #[arg(short = 'p', long)]
    pub pretty: bool,
}

/// Parses statements in parallel. Pages are all parsed in an (optimized) sequence.
/// (I don't know how to do nested tasks right now).
async fn parse_statements_parallel(
    file_paths: &Vec<PathBuf>,
) -> Result<Vec<StatementFmvs>, Vec<SError>> {
    let start = std::time::Instant::now();

    let mut handles = Vec::with_capacity(file_paths.len());
    for file_path in file_paths {
        let fp_clone = file_path.clone();
        let handle =
            async_std::task::spawn(async move { parse_statement(&fp_clone, false) });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(file_paths.len());
    for handle in handles {
        results.push(handle.await);
    }

    let mut statements = Vec::with_capacity(results.len());
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(st) => {
                statements.push(st);
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    tracing::debug!("parse_statements_async took {:?}", start.elapsed());
    if errors.is_empty() {
        Ok(statements)
    } else {
        Err(errors)
    }
}

/// Parses statements, and loads pages asynchronously, but processes
/// statements sequentially (I don't know how to do nested tasks right now).
fn parse_statements_pages_parallel(
    file_paths: &Vec<PathBuf>,
) -> Result<Vec<StatementFmvs>, Vec<SError>> {
    let start = std::time::Instant::now();

    let mut statements = Vec::<StatementFmvs>::new();
    for file_path in file_paths {
        let statement_fmvs =
            parse_statement(&file_path, true).map_err(|e| vec![e])?;
        statements.push(statement_fmvs);
    }

    tracing::debug!("parse_statements_async took {:?}", start.elapsed());
    Ok(statements)
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();

    crate::tracing::setup_tracing();

    let mut statements =
        if crate::util::sys::env_var_non_empty("PARALLEL_STATEMENTS") {
            // This optimization path is not recommended, at least from some
            // limited testing. Though the actual parse step is noticeably faster
            // here, the overall runtime (at least on a 4-core laptop from 2015) is
            // slower. This may because loading all of the statements in parallel
            // creates an IO bottleneck. We could maybe adjust this based on certain
            // parameters, but without more machine variety to test on, how to do
            // that isn't clear.
            async_std::task::block_on(parse_statements_parallel(&args.files))
        } else {
            parse_statements_pages_parallel(&args.files)
        }
        .map_err(|e| eprintln!("{}", e.join("\n")))?;

    // Month is not Ord for some reason, so mock out a date for now.
    statements.sort_by_cached_key(|st| st.month_date);

    let table = render_table_for_statements(&statements);

    let write_res = if args.pretty {
        crate::app::outfmt::text::TextWriter::new(WriteHandle::stdout_write_handle())
            .print_render_table(
                crate::app::outfmt::model::OutputType::Raw,
                "FMVs",
                &table,
            )
    } else {
        crate::app::outfmt::csv::CsvWriter::new_to_writer(
            WriteHandle::stdout_write_handle(),
        )
        .print_render_table(
            crate::app::outfmt::model::OutputType::Raw,
            "FMVs",
            &table,
        )
    };

    if let Err(e) = write_res {
        eprintln!("Error writing table: {e}");
        return Err(());
    }

    Ok(())
}

// MARK: tests

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::testlib::{assert_big_struct_eq, assert_vec_eq};

    use super::sm::FmvParseSm;
    use super::{parse_fmvs_from_page, parse_statement_text, Fmv};

    fn s(st: &str) -> String {
        st.to_string()
    }

    #[test]
    fn test_security_text_to_fmv() {
        // We can get some interestingly long names with GICs
        assert_eq!(
            FmvParseSm::security_text_to_fmv(
                " FOO BAR 5% 01/01/2024 (FOOBAR) 99   100.0 1,234.0"
            )
            .unwrap(),
            Fmv {
                security_desc: s("FOO BAR 5% 01/01/2024 (FOOBAR) 99"),
                allocation: dec!(100.00),
                fmv: dec!(1234)
            }
        );

        assert_eq!(
            FmvParseSm::security_text_to_fmv("   X   55.55 0").unwrap(),
            Fmv {
                security_desc: s("X"),
                allocation: dec!(55.55),
                fmv: dec!(0)
            }
        );

        // Various errors

        let errs = vec![
            FmvParseSm::security_text_to_fmv("X").unwrap_err(),
            FmvParseSm::security_text_to_fmv("X 1,000 50").unwrap_err(),
            FmvParseSm::security_text_to_fmv("X 1.00.0 50").unwrap_err(),
            FmvParseSm::security_text_to_fmv("X 50 1.1.1").unwrap_err(),
        ];
        assert_vec_eq(errs, vec![
            s("Unable to parse allocation and FMV from \"X\""),
            s("Unable to parse allocation and FMV from \"X 1,000 50\""),
            s("Unable to parse allocation from \"1.00.0\": Invalid decimal: two decimal points"),
            s("Unable to parse FMV from \"1.1.1\": Invalid decimal: two decimal points"),
        ]);
    }

    #[test]
    fn test_parse_fmvs_from_page_basic() {
        let (fmvs, total) = parse_fmvs_from_page(
            "
            ALLOCATION (%)² MARKET VALUE ($)³
            100.0 0.0",
        )
        .unwrap();
        assert_eq!(fmvs, vec![]);
        assert_eq!(total, dec!(0));

        let (fmvs, total) = parse_fmvs_from_page(
            "
            ALLOCATION (%)² MARKET VALUE ($)³

            ■ BLABLA ETF (BLABLA) 80.0 80,000.0

            ■ SOME GIC 01/01/2024
            4.00% 1Y DUE 01/01/2024  INT  4.000% (XXXXXX) 5.0 5,000.1

            ■ ANOTHER GIC 01/01/2025
            5.00% 2Y CPD DUE 01/01/2025  INT  5.00%
            (YYYYYY)
            15.0 15,000.0

            100.0 100,000.01
            ",
        )
        .unwrap();
        assert_vec_eq(
            fmvs,
            vec![
                Fmv {
                    security_desc: s("BLABLA ETF (BLABLA)"),
                    allocation: dec!(80),
                    fmv: dec!(80000),
                },
                Fmv {
                    security_desc: s(
                        "SOME GIC 01/01/2024 4.00% 1Y DUE 01/01/2024  \
                                    INT  4.000% (XXXXXX)",
                    ),
                    allocation: dec!(5),
                    fmv: dec!(5000.1),
                },
                Fmv {
                    security_desc: s("ANOTHER GIC 01/01/2025 5.00% 2Y CPD DUE \
                                    01/01/2025  INT  5.00% (YYYYYY)"),
                    allocation: dec!(15),
                    fmv: dec!(15000),
                },
            ],
        );
        assert_eq!(total, dec!(100000.01));

        // Test a single security, which fakes out the 100% line.
        let (fmvs, total) = parse_fmvs_from_page(
            "
            ALLOCATION (%)² MARKET VALUE ($)³

            ■ SOME GIC 01/01/2024
            4.00% 1Y DUE 01/01/2024  INT  4.000% (XXXXXX)
            100.0 99,999.99

            100.0 100,000.00
            ",
        )
        .unwrap();
        assert_vec_eq(
            fmvs,
            vec![Fmv {
                security_desc: s("SOME GIC 01/01/2024 4.00% 1Y DUE 01/01/2024  \
                                    INT  4.000% (XXXXXX)"),
                allocation: dec!(100),
                fmv: dec!(99999.99),
            }],
        );
        assert_eq!(total, dec!(100000.00));
    }

    #[test]
    fn test_parse_fmvs_from_page_errors() {
        // Nothing
        let err = parse_fmvs_from_page("").unwrap_err();
        assert_eq!(err, "No header or allocation total line found");

        // No header
        let err = parse_fmvs_from_page(
            "
        100.0 0.0",
        )
        .unwrap_err();
        assert_eq!(err, "No header or allocation total line found");

        // No footer or securities
        let err =
            parse_fmvs_from_page("ALLOCATION (%) MARKET VALUE ($)").unwrap_err();
        assert_eq!(err, "No header or allocation total line found");

        // No footer
        let err = parse_fmvs_from_page(
            "
            ALLOCATION (%)² MARKET VALUE ($)³

            ■ BLABLA ETF (BLABLA) 80.0 80,000.0",
        )
        .unwrap_err();
        assert_eq!(err, "No header or allocation total line found");

        // Unterminated security / missing value(s)
        let err = parse_fmvs_from_page(
            "
            ALLOCATION (%)² MARKET VALUE ($)³
            ■ FOO ETF (FOO)
            ■ BAR ETF (BAR) 80.0 1,000
            100.0 50,000.0
            ",
        )
        .unwrap_err();
        assert_eq!(
            err,
            "Unable to parse allocation and FMV from \"FOO ETF (FOO)\""
        );
    }

    #[test]
    fn test_parse_statement_text() {
        let month_page = s("Leading garbage
            Account #:  1234 Current month:  February 28, 2024 trailing garbage");
        let fmv_page = s(" Leading garbage
        Securities Owned

        Combined in (CAD)¹
        ALLOCATION (%)² MARKET VALUE ($)³

        ■ BLABLA ETF (BLABLA) 80.0 80,000.0

        100.0 100,000.01
        ");

        let statement =
            parse_statement_text(vec![month_page.clone(), fmv_page.clone()].iter())
                .unwrap();

        let exp_fmvs = super::StatementFmvs {
            month_date: time::Date::from_calendar_date(
                2024,
                time::Month::February,
                28,
            )
            .unwrap(),
            fmvs: vec![Fmv {
                security_desc: s("BLABLA ETF (BLABLA)"),
                allocation: dec!(80),
                fmv: dec!(80000),
            }],
            total: dec!(100000.01),
        };

        assert_big_struct_eq(&statement, &exp_fmvs);

        // Two FMV pages for some reason (second is ignored)
        let statement = parse_statement_text(
            vec![
                month_page.clone(),
                fmv_page.clone(),
                s("Securities Owned Combined in (CAD)
            ALLOCATION (%) MARKET VALUE ($)
            ■ FOO ETF (FOO) 80.0 80,000.0
            100.0 100,000.01
            "),
            ]
            .iter(),
        )
        .unwrap();

        assert_big_struct_eq(&statement, &exp_fmvs);

        // Month page second, for whatever reason
        assert_eq!(
            parse_statement_text(vec![fmv_page.clone(), month_page.clone(),].iter())
                .unwrap_err(),
            s("Could not find month")
        );
    }

    #[test]
    fn test_sec_desc_abbrev() {
        assert_eq!(super::sec_desc_abbrev("Bla bla bla (BXX12)"), "BXX12");
        assert_eq!(super::sec_desc_abbrev("Bla bla bla (BXX12) "), "BXX12");
        assert_eq!(super::sec_desc_abbrev("Bla bla bla"), "Bbb");
        assert_eq!(
            super::sec_desc_abbrev("Bla bla bla (some paranthetical)"),
            "Bbb"
        );
        assert_eq!(
            super::sec_desc_abbrev("Bl Xi Z (some paranthetical)"),
            "BlXiZ"
        );
        assert_eq!(
            super::sec_desc_abbrev("Bla bla (some paranthetical)"),
            "Blabla"
        );
        assert_eq!(super::sec_desc_abbrev("Bla (some paranthetical)"), "Bla");
    }
}
