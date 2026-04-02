use acb::{
    peripheral::tx_export_convert_impl::{run_with_args, Args},
    testlib::assert_vec_eq,
    util::rw::{StrReader, StringBuffer, WriteHandle},
};
use clap::Parser;

fn run_and_get_output(args: Args) -> (Result<(), ()>, String, String) {
    #[allow(unused_mut)] // Some linting error here
    let (out_w, mut out_b) = WriteHandle::string_buff_write_handle();
    #[allow(unused_mut)] // Some linting error here
    let (err_w, mut err_b) = WriteHandle::string_buff_write_handle();
    let res = run_with_args(args, out_w, err_w);
    let mut out_b_ = out_b.borrow_mut();
    let mut err_b_ = err_b.borrow_mut();
    (res, out_b_.export_string(), err_b_.export_string())
}

fn parse_qt_args(scenario: &str, flags: Vec<&str>) -> Args {
    let path =
        format!("./tests/data/questrade_scenarios/{}/Activities_{}.xlsx", scenario, scenario);
    let mut args: Vec<String> = vec!["tx-export-convert".to_string()];
    args.extend(flags.iter().map(|s| s.to_string()));
    args.push(path);
    Args::parse_from(args)
}

fn padded_csv_text_to_unpadded(padded_csv_text: &str) -> String {
    let r = StrReader::from(padded_csv_text);
    let mut csv_r = csv::ReaderBuilder::new().has_headers(false).from_reader(r);
    let mut sbuf = StringBuffer::new();
    {
        let mut csv_w =
            csv::WriterBuilder::new().has_headers(false).from_writer(&mut sbuf);
        for rec_res in csv_r.records() {
            let trimmed_record: Vec<String> = rec_res
                .unwrap()
                .iter()
                .map(|v| v.to_string().trim().to_string())
                .collect();
            csv_w.write_record(trimmed_record).unwrap();
        }
    }
    sbuf.export_string()
}

fn remove_columns(csv_text: &str, col_indexes: &Vec<usize>) -> String {
    let r = StrReader::from(csv_text);
    let mut csv_r = csv::ReaderBuilder::new().has_headers(false).from_reader(r);
    let mut sbuf = StringBuffer::new();
    {
        let mut csv_w =
            csv::WriterBuilder::new().has_headers(false).from_writer(&mut sbuf);
        for rec_res in csv_r.records() {
            let filtered_record: Vec<String> = rec_res
                .unwrap()
                .into_iter()
                .enumerate()
                .filter(|(i, _)| !col_indexes.contains(i))
                .map(|(_, v)| v.to_string())
                .collect();
            csv_w.write_record(filtered_record).unwrap();
        }
    }
    sbuf.export_string()
}

fn include_lines(s: &str, include_pattern: &str) -> String {
    let pattern = regex::Regex::new(include_pattern).unwrap();
    s.split("\n")
        .into_iter()
        .filter(|l| pattern.is_match(l) || l.contains("security"))
        .collect::<Vec<&str>>()
        .join("\n")
}

fn exclude_lines(s: &str, exclude_pattern: &str) -> String {
    let pattern = regex::Regex::new(exclude_pattern).unwrap();
    s.split("\n")
        .into_iter()
        .filter(|l| !pattern.is_match(l) || l.contains("security"))
        .collect::<Vec<&str>>()
        .join("\n")
}

fn verify_multiline(multiline: &str, exp: &str) {
    let exp_lines = exp.split("\n").into_iter().collect();
    let lines = multiline.split("\n").into_iter().collect();
    assert_vec_eq(lines, exp_lines);
}

fn verify_csv(csv_str: &str, exp_csv_str: &str) {
    let unpadded_exp = padded_csv_text_to_unpadded(exp_csv_str);
    verify_multiline(csv_str, &unpadded_exp);
}

#[test]
fn test_txs_basic_and_ignored_actions() {
    let (res, out, err) =
        run_and_get_output(parse_qt_args("basic", vec!["--account", "."]));

    assert_eq!("", &err);
    res.unwrap();
    let exp_csv = "\
    security,trade date,settlement date,action,shares,amount/share,commission,currency,affiliate,memo
    CCO     ,2023-01-04,2023-01-05     ,Buy   ,2     ,10.00       ,3.99      ,CAD     ,Default (R),Questrade Individual TFSA 10000001
    DLR.TO  ,2023-01-06,2023-01-07     ,Buy   ,3     ,1.00        ,0.00      ,CAD     ,Default (R),Questrade Individual RRSP 10000002
    CCO     ,2023-01-08,2023-01-09     ,Buy   ,4     ,10.00       ,1.00      ,CAD     ,Default    ,Questrade Individual margin 10000003
    USD.FX  ,2023-01-10,2023-01-10     ,Sell  ,51    ,1.00        ,0.00      ,USD     ,Default (R),Questrade Individual TFSA 10000001; from DLR.TO Buy
    DLR.TO  ,2023-01-10,2023-01-11     ,Buy   ,5     ,10.00       ,1.00      ,USD     ,Default (R),Questrade Individual TFSA 10000001; H038778 AKA DLR.U.TO
    USD.FX  ,2023-01-12,2023-01-12     ,Sell  ,60    ,1.00        ,0.00      ,USD     ,Default (R),Questrade Individual RRSP 10000002; from UCO Buy
    UCO     ,2023-01-12,2023-01-13     ,Buy   ,6     ,10.00       ,0.00      ,USD     ,Default (R),Questrade Individual RRSP 10000002
    USD.FX  ,2023-01-14,2023-01-14     ,Sell  ,71    ,1.00        ,0.00      ,USD     ,Default    ,Questrade Individual margin 10000003; from UCO Buy
    UCO     ,2023-01-14,2023-01-15     ,Buy   ,7     ,10.00       ,1.00      ,USD     ,Default    ,Questrade Individual margin 10000003
    CCO     ,2023-01-16,2023-01-17     ,Sell  ,8     ,10.00       ,1.00      ,CAD     ,Default (R),Questrade Individual TFSA 10000001
    CCO     ,2023-01-18,2023-01-19     ,Sell  ,9     ,10.00       ,1.00      ,CAD     ,Default (R),Questrade Individual RRSP 10000002
    CCO     ,2023-01-20,2023-01-21     ,Sell  ,10    ,10.00       ,1.00      ,CAD     ,Default    ,Questrade Individual margin 10000003
    USD.FX  ,2023-01-22,2023-01-22     ,Buy   ,109   ,1.00        ,0.00      ,USD     ,Default (R),Questrade Individual TFSA 10000001; from UCO Sell
    UCO     ,2023-01-22,2023-01-23     ,Sell  ,11    ,10.00       ,1.00      ,USD     ,Default (R),Questrade Individual TFSA 10000001
    USD.FX  ,2023-01-24,2023-01-24     ,Buy   ,119   ,1.00        ,0.00      ,USD     ,Default (R),Questrade Individual RRSP 10000002; from UCO Sell
    UCO     ,2023-01-24,2023-01-25     ,Sell  ,12    ,10.00       ,1.00      ,USD     ,Default (R),Questrade Individual RRSP 10000002
    USD.FX  ,2023-01-26,2023-01-26     ,Buy   ,129   ,1.00        ,0.00      ,USD     ,Default    ,Questrade Individual margin 10000003; from UCO Sell
    UCO     ,2023-01-26,2023-01-27     ,Sell  ,13    ,10.00       ,1.00      ,USD     ,Default    ,Questrade Individual margin 10000003
    USD.FX  ,2023-01-28,2023-01-28     ,Buy   ,30.5  ,1.00        ,0.00      ,USD     ,Default    ,Questrade Individual margin 10000003; DIV from UCO
    UCO     ,2023-01-28,2023-01-29     ,Buy   ,20    ,0.00        ,0.00      ,USD     ,Default    ,Questrade Individual margin 10000003; From DIS action.
    UCO     ,2023-01-28,2023-01-29     ,Sell  ,19    ,0.00        ,0.00      ,USD     ,Default    ,Questrade Individual margin 10000003; From LIQ action.\
    ";

    verify_csv(&out, &exp_csv);

    // Test filters
    let (res, out, err) = run_and_get_output(parse_qt_args(
        "basic",
        vec!["--account", "margin"],
    ));
    assert_eq!("", &err);
    res.unwrap();
    verify_csv(
        &out,
        &remove_columns(&include_lines(&exp_csv, "margin"), &vec![AFFIL_COL]),
    );

    let (res, out, err) = run_and_get_output(parse_qt_args(
        "basic",
        vec!["--account", "margin", "--security", "UCO"],
    ));
    assert_eq!("", &err);
    res.unwrap();
    const AFFIL_COL: usize = 8;
    verify_csv(
        &out,
        &remove_columns(&include_lines(&exp_csv, r"UCO.*margin"), &vec![AFFIL_COL]),
    );

    let (res, out, err) = run_and_get_output(parse_qt_args(
        "basic",
        vec!["--account", ".", "--no-fx"],
    ));
    assert_eq!("", &err);
    res.unwrap();
    verify_csv(&out, &exclude_lines(&exp_csv, r"USD\.FX"));
}

// All pages in order:
// TXs, FXTs,  TX Errors, FXT Errors, Sorting

const BASIC_HEADER: &str =
"security,trade date,settlement date,action,shares,amount/share,commission,currency,memo";

#[test]
fn test_fxt_basic() {
    let (res, out, err) =
        run_and_get_output(parse_qt_args("fxt", vec!["--account", "."]));

    assert_eq!("", &err);
    res.unwrap();
    let exp_csv = "\
    security,trade date,settlement date,action,shares,amount/share,commission,currency,exchange rate,memo
    USD.FX  ,2023-02-05,2023-02-05     ,Buy   ,100   ,1.00        ,0.00       ,USD    ,1.3          ,Questrade Individual margin 10000003; FXT
    USD.FX  ,2023-02-05,2023-02-05     ,Sell  ,200   ,1.00        ,0.00       ,USD    ,1.25         ,Questrade Individual margin 10000003; FXT\
    ";

    verify_csv(&out, &exp_csv);

    // Filter all FXTs
    let (res, out, err) = run_and_get_output(parse_qt_args(
        "fxt",
        vec!["--account", ".", "--no-fx"],
    ));
    assert_eq!("", &err);
    res.unwrap();
    verify_csv(&out, BASIC_HEADER);
}

#[test]
fn test_tx_errors() {
    let (res, out, err) =
        run_and_get_output(parse_qt_args("tx_errors", vec!["--account", "."]));

    res.unwrap_err();
    // Partial shares are allowed
    verify_csv(&out, "\
    security,trade date,settlement date,action,shares,amount/share,commission,currency,affiliate,memo
    BAR,2023-01-04,2023-01-05,Buy,2.5,10.00,3.99,CAD,Default (R),Questrade Individual TFSA 10000001\
    ");

    let exp_errs = "\
Errors: - Row 2: Unable to parse date \"2023-1-7\"
 - Row 3: Unable to parse date \"\"
 - Row 4: Unable to parse date \"2023-1-8\"
 - Row 5: Unable to parse date \"\"
 - Row 6: Unrecognized transaction action XXX
 - Row 7: Symbol was empty
 - Row 10: value in Quantity was empty
 - Row 11: value in Quantity was empty
 - Row 12: Unable to parse number from \"abc\" in Quantity: Invalid decimal: unknown character
 - Row 13: Unable to parse number from \"abc\" in Quantity: Invalid decimal: unknown character
 - Row 14: value in Price was empty
 - Row 15: value in Price was empty
 - Row 16: Unable to parse number from \"abc\" in Price: Invalid decimal: unknown character
 - Row 17: Unable to parse number from \"abc\" in Price: Invalid decimal: unknown character
 - Row 18: value in Commission was empty
 - Row 19: value in Commission was empty
 - Row 20: Unable to parse number from \"abc\" in Commission: Invalid decimal: unknown character
 - Row 21: Unable to parse number from \"abc\" in Commission: Invalid decimal: unknown character
";
    verify_multiline(&err, &exp_errs);
}

#[test]
fn test_fxt_errors() {
    let (res, out, err) =
        run_and_get_output(parse_qt_args("fxt_errors", vec!["--account", "."]));

    res.unwrap_err();
    verify_csv(&out, "\
    security,trade date,settlement date,action,shares,amount/share,commission,currency,affiliate,memo
    FOO     ,2023-01-04,2023-01-05     ,Buy   ,2     ,10.00       ,3.99      ,CAD     ,Default (R),Questrade Individual TFSA 10000001\
    ");

    let exp_errs = "\
Errors: - Row 5: Both FXTs have positive amounts
 - Row 7: Both FXTs have negative amounts
 - Row 9: FXTs not supported between CAD and CAD. Exactly one currency must be CAD.
 - Row 11: FXTs not supported between USD and USD. Exactly one currency must be CAD.
 - Row 13: FX currency UNK not supported
 - Row 15: FX currency UNK not supported
 - Row 17: FXTs not supported between UNK and USD. Exactly one currency must be CAD.
 - Row 19: FXTs not supported between USD and UNK. Exactly one currency must be CAD.
 - Row 21: FXTs not supported between UNK2 and UNK1. Exactly one currency must be CAD.
 - Row 22: Unpaired FXT
";
    verify_multiline(&err, &exp_errs);
}

#[test]
fn test_sort() {
    let (res, out, err) =
        run_and_get_output(parse_qt_args("sorting", vec!["--account", "."]));

    assert_eq!("", &err);
    res.unwrap();
    let exp_csv = "\
    security,trade date,settlement date,action,shares,amount/share,commission,currency,exchange rate,memo
    UCO,2023-01-12,2023-01-12,Buy,1,10.00,0.00,USD,,Questrade Individual margin 10000003
    UCO,2023-01-12,2023-01-12,Sell,3,10.00,0.00,USD,,Questrade Individual margin 10000003
    USD.FX,2023-01-12,2023-01-12,Buy,100,1.00,0.00,USD,1.3,Questrade Individual margin 10000003; FXT
    USD.FX,2023-01-12,2023-01-12,Buy,30,1.00,0.00,USD,,Questrade Individual margin 10000003; from UCO Sell
    USD.FX,2023-01-12,2023-01-12,Buy,40,1.00,0.00,USD,,Questrade Individual margin 10000003; from UCO Sell
    USD.FX,2023-01-12,2023-01-12,Sell,200,1.00,0.00,USD,1.25,Questrade Individual margin 10000003; FXT
    USD.FX,2023-01-12,2023-01-12,Sell,10,1.00,0.00,USD,,Questrade Individual margin 10000003; from UCO Buy
    USD.FX,2023-01-12,2023-01-12,Sell,20,1.00,0.00,USD,,Questrade Individual margin 10000003; from UCO Buy
    UCO,2023-01-12,2023-01-13,Buy,2,10.00,0.00,USD,,Questrade Individual margin 10000003
    UCO,2023-01-12,2023-01-13,Sell,4,10.00,0.00,USD,,Questrade Individual margin 10000003
    UCO,2023-01-13,2023-01-13,Sell,5,10.00,0.00,USD,,Questrade Individual margin 10000003
    USD.FX,2023-01-13,2023-01-13,Buy,50,1.00,0.00,USD,,Questrade Individual margin 10000003; from UCO Sell \
    ";

    verify_csv(&out, &exp_csv);
}

// ---- RBC Direct Investing tests ----

fn parse_rbc_args(mut flags: Vec<&str>) -> Args {
    let mut args = vec!["tx-export-convert", "-b", "rbc-di"];
    args.append(&mut flags);
    args.push("./tests/data/RBC_DI_Test_Export.csv");
    Args::parse_from(args)
}

#[test]
fn test_rbc_di_basic_buy_sell() {
    let (res, out, err) = run_and_get_output(parse_rbc_args(vec![]));

    // Should have warnings (not errors) for RoC and Reorganization
    res.unwrap();
    assert!(err.contains("Warnings:"));
    assert!(!err.contains("Errors:"));
    assert!(err.contains("Return of Capital"));
    assert!(err.contains("Reorganization"));
    assert!(err.contains("not automatically converted"));

    // Verify the actual buy/sell transactions
    let exp_csv = "\
    security  ,trade date ,settlement date,action,shares,amount/share,commission,currency,affiliate  ,memo
    XEQT      ,2025-01-10 ,2025-01-14     ,Buy   ,10    ,40.00       ,9.95      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    ZNQ       ,2025-02-05 ,2025-02-07     ,Buy   ,5     ,100.00      ,0.00      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    XEQT      ,2025-03-15 ,2025-03-18     ,Sell  ,4     ,42.50       ,9.95      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    XEQT      ,2025-09-10 ,2025-09-12     ,Buy   ,8     ,41.25       ,0.00      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    ZNQ       ,2025-10-05 ,2025-10-08     ,Sell  ,3     ,55.00       ,9.90      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    VFV       ,2025-11-20 ,2025-11-24     ,Buy   ,20    ,120.00      ,9.95      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    VFV       ,2025-12-15 ,2025-12-17     ,Sell  ,10    ,122.50      ,9.95      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678\
    ";

    verify_csv(&out, &exp_csv);
}

#[test]
fn test_rbc_di_security_filter() {
    let (res, out, err) =
        run_and_get_output(parse_rbc_args(vec!["--security", "XEQT"]));

    assert_ne!("", &err);
    // Still has warnings for RoC/Reorg, but they are for different securities
    res.unwrap();

    let exp_csv = "\
    security,trade date ,settlement date,action,shares,amount/share,commission,currency,affiliate  ,memo
    XEQT    ,2025-01-10 ,2025-01-14     ,Buy   ,10    ,40.00       ,9.95      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    XEQT    ,2025-03-15 ,2025-03-18     ,Sell  ,4     ,42.50       ,9.95      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678
    XEQT    ,2025-09-10 ,2025-09-12     ,Buy   ,8     ,41.25       ,0.00      ,CAD     ,Default (R),RBC Direct Investing TFSA 12345678\
    ";

    verify_csv(&out, &exp_csv);
}

#[test]
fn test_rbc_di_year_filter() {
    // Filter to only 2025 Jan-Mar by using a custom date range via the year flag.
    // The year flag filters by settlement date year, so all 2025 txs should match.
    let (res, out, err) = run_and_get_output(parse_rbc_args(vec!["--year", "2025"]));

    assert_ne!("", &err);
    // Should still have the same transactions since all are in 2025
    res.unwrap(); // warnings still present but not errors

    // Just verify the output contains buy/sell transactions
    assert!(out.contains("XEQT"));
    assert!(out.contains("ZNQ"));
    assert!(out.contains("VFV"));
}

#[test]
fn test_rbc_di_sorted_by_settlement_date() {
    let (res, out, _err) = run_and_get_output(parse_rbc_args(vec![]));
    res.unwrap(); // warnings only, no errors

    // Parse dates from output and verify they're sorted
    let r = StrReader::from(out.as_str());
    let mut csv_r = csv::ReaderBuilder::new().from_reader(r);
    let mut settlement_dates: Vec<String> = Vec::new();
    for rec_res in csv_r.records() {
        let rec = rec_res.unwrap();
        settlement_dates.push(rec[2].to_string());
    }
    let mut sorted = settlement_dates.clone();
    sorted.sort();
    assert_eq!(settlement_dates, sorted);
}

fn parse_rbc_error_args(mut flags: Vec<&str>) -> Args {
    let mut args = vec!["tx-export-convert", "-b", "rbc-di"];
    args.append(&mut flags);
    args.push("./tests/data/RBC_DI_Test_Export_Errors.csv");
    Args::parse_from(args)
}

#[test]
fn test_rbc_di_errors() {
    let (res, out, err) = run_and_get_output(parse_rbc_error_args(vec![]));

    res.unwrap_err();
    assert!(err.contains("Errors:"), "err: {}", err);
    assert!(
        err.contains("Unrecognized activity type \"UnknownActivity\""),
        "err: {}",
        err
    );

    // The valid buy row before the error row should still appear in output
    verify_csv(
        &out,
        "\
    security,trade date,settlement date,action,shares,amount/share,commission,currency,affiliate,memo
    XEQT,2025-01-10,2025-01-14,Buy,10,40.00,9.95,CAD,Default (R),RBC Direct Investing TFSA 11111111\
    ",
    );
}
