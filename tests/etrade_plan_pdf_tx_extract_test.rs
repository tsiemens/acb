mod common;

use std::{fs, io::Read, path::Path, path::PathBuf};

use acb::{
    peripheral::etrade_plan_pdf_tx_extract_cmd::{run_with_args, Args},
    util::rw::WriteHandle,
};

use common::run_test;

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

fn do_test_scenario(scenario_variant_dir: &Path) {
    let is_xlsx_variant = scenario_variant_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("xlsx_"))
        .unwrap_or(false);

    let mut files: Vec<std::path::PathBuf> = fs::read_dir(scenario_variant_dir)
        .unwrap()
        .filter(|rd| rd.is_ok())
        .map(|rd| rd.unwrap().path())
        .filter(|p| {
            let s = p.display().to_string();
            s.ends_with(".txt") || (is_xlsx_variant && s.ends_with(".xlsx"))
        })
        .collect();
    files.sort();
    assert_ne!(files, Vec::<std::path::PathBuf>::new());

    let args = Args {
        files,
        pretty: false,
        extract_only: false,
        debug: false,
        no_fx: true,
        no_sell_to_cover_pair: is_xlsx_variant,
        year: None,
        config: None,
    };
    let (res, out, err) = run_and_get_output(args);
    assert!(res.is_ok(), "res={:?} output={}, err={}", res, out, err);
    assert_eq!(err, "");
    assert_ne!(out, "");

    let variant_exp = scenario_variant_dir.join("expected_output.csv");
    let exp_path = if variant_exp.exists() {
        variant_exp
    } else {
        scenario_variant_dir.join("../expected_output.csv")
    };
    let mut exp_out = String::new();
    fs::File::open(&exp_path).unwrap().read_to_string(&mut exp_out).unwrap();

    acb::testlib::assert_vec_eq(
        out.split("\n").collect(),
        exp_out.split("\n").collect(),
    );
}

#[test]
fn test_etrade_scenarios() {
    common::test_init_tracing();
    let potential_variants = vec![
        "dfltpdf",
        "lopdf",
        "pypdf",
        "xlsx_dfltpdf",
        "xlsx_lopdf",
        "xlsx_pypdf",
    ];

    for sdr in fs::read_dir(Path::new("./tests/data/etrade_scenarios")).unwrap() {
        let scenario_dir = sdr.unwrap().path();
        for variant in &potential_variants {
            let variant_dir = scenario_dir.join(variant);
            if variant_dir.exists() {
                run_test(&variant_dir.display().to_string(), || {
                    do_test_scenario(&variant_dir);
                })
            }
        }
    }
}

#[test]
fn test_etrade_pdf_parse_error() {
    let args = Args {
        files: vec![PathBuf::from(
            "./tests/data/etrade_scenarios/parse_errors/bad_trade_conf.txt",
        )],
        pretty: false,
        extract_only: false,
        debug: false,
        no_fx: true,
        no_sell_to_cover_pair: false,
        year: None,
        config: None,
    };
    let (res, _out, err) = run_and_get_output(args);
    res.unwrap_err();
    assert!(
        err.contains("Cannot categorize layout of PDF"),
        "err: {}",
        err
    );
}

#[test]
fn test_etrade_scenario_with_config() {
    // Use 2024_with_manual_sells/dfltpdf: the trade-confirmation account
    // "123-XXX789-111"/"12345678" is bound to "Alice" in alt-config.json.
    let variant_dir =
        Path::new("./tests/data/etrade_scenarios/2024_with_manual_sells/dfltpdf");
    let mut files: Vec<PathBuf> = fs::read_dir(variant_dir)
        .unwrap()
        .filter_map(|rd| rd.ok())
        .map(|rd| rd.path())
        .filter(|p| p.display().to_string().ends_with(".txt"))
        .collect();
    files.sort();
    assert_ne!(files, Vec::<PathBuf>::new());

    let args = Args {
        files,
        pretty: false,
        extract_only: false,
        debug: false,
        no_fx: true,
        no_sell_to_cover_pair: false,
        year: None,
        config: Some(PathBuf::from("./tests/data/alt-config.json")),
    };
    let (res, out, err) = run_and_get_output(args);
    assert!(res.is_ok(), "res={:?} out={} err={}", res, out, err);
    assert_eq!(err, "");

    // Derive expected output from the no-config expected CSV: splice an
    // "affiliate" column between currency (col 7) and memo (col 8).
    let base_exp_path = variant_dir.join("../expected_output.csv");
    let mut base_exp = String::new();
    fs::File::open(&base_exp_path)
        .unwrap()
        .read_to_string(&mut base_exp)
        .unwrap();
    let exp_out = splice_affiliate_column(&base_exp, |_cells| "Alice");

    acb::testlib::assert_vec_eq(
        out.split("\n").collect(),
        exp_out.split("\n").collect(),
    );
}

/// Inserts an `affiliate` column between `currency` and `memo` in a CSV
/// string. The header row gets the literal "affiliate"; the value for each
/// data row is chosen by `row_value`, which receives the pre-splice cell
/// slice.
fn splice_affiliate_column<F>(csv_str: &str, row_value: F) -> String
where
    F: Fn(&[&str]) -> &'static str,
{
    let mut out_lines: Vec<String> = Vec::new();
    for (i, line) in csv_str.split('\n').enumerate() {
        if line.is_empty() {
            out_lines.push(String::new());
            continue;
        }
        let cells: Vec<&str> = line.split(',').collect();
        // Column layout (pre-splice): security, trade date, settlement date,
        // action, shares, amount/share, commission, currency, memo
        // Splice at index 8 (right before memo).
        let insert_val: &str = if i == 0 {
            "affiliate"
        } else {
            row_value(&cells)
        };
        let mut new_cells: Vec<String> =
            cells[..8].iter().map(|s| s.to_string()).collect();
        new_cells.push(insert_val.to_string());
        new_cells.extend(cells[8..].iter().map(|s| s.to_string()));
        out_lines.push(new_cells.join(","));
    }
    out_lines.join("\n")
}

#[test]
fn test_etrade_xlsx_benefits_parse_error() {
    let args = Args {
        files: vec![PathBuf::from(
            "./tests/data/etrade_scenarios/parse_errors/bad_benefits.xlsx",
        )],
        pretty: false,
        extract_only: false,
        debug: false,
        no_fx: true,
        no_sell_to_cover_pair: false,
        year: None,
        config: None,
    };
    let (res, _out, err) = run_and_get_output(args);
    res.unwrap_err();
    assert!(err.contains("Failed to parse"), "err: {}", err);
}
