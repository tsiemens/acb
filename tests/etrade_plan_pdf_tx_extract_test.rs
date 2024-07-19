mod common;

use std::{fs, io::Read, path::Path};

use acb::{peripheral::etrade_plan_pdf_tx_extract_impl::{run_with_args, Args}, util::rw::WriteHandle};

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
    let files: Vec<std::path::PathBuf> = fs::read_dir(scenario_variant_dir).unwrap()
        .filter(|rd| rd.is_ok())
        .map(|rd| rd.unwrap().path())
        .filter(|p| p.display().to_string().ends_with(".txt"))
        .collect();
    assert_ne!(files, Vec::<std::path::PathBuf>::new());

    let args = Args {
        files: files,
        pretty: false,
        extract_only: false,
        debug: false,
    };
    let (res, out, err) = run_and_get_output(args);
    assert!(res.is_ok());
    assert_eq!(err, "");
    assert_ne!(out, "");

    let mut exp_out = String::new();
    fs::File::open(scenario_variant_dir.join("../expected_output.csv")).unwrap()
        .read_to_string(&mut exp_out).unwrap();

    acb::testlib::assert_vec_eq(out.split("\n").collect(),
                                exp_out.split("\n").collect());
}

#[test]
fn test_etrade_scenarios() {
    let potential_variants = vec!["dfltpdf", "lopdf", "pypdf"];

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