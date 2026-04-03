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
    };
    let (res, _out, err) = run_and_get_output(args);
    res.unwrap_err();
    assert!(err.contains("Failed to parse"), "err: {}", err);
}
