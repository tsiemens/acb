use acb::peripheral::csv_merge_impl::merge_csv_files;

fn fixture_path(name: &str) -> String {
    std::path::Path::new("./tests/data/csv_merge")
        .join(name)
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_merge_preserves_first_file_column_order() {
    let mut output = Vec::new();
    merge_csv_files(
        &[
            fixture_path("all-full-2024-like.csv"),
            fixture_path("broker1.csv"),
            fixture_path("broker2.csv"),
        ],
        &mut output,
    )
    .unwrap();
    assert_eq!(
        "\
security,trade date,settlement date,action,shares,amount/share,commission,split ratio,currency,memo,Exchange Rate
AAA,2024-01-02,2024-01-04,Buy,3,373.305,1.23,,USD,Broker legacy buy,
BBB,2024-02-12,2024-02-14,Sell,4,143.21,2.34,,USD,Broker legacy sell,
CCC,2024-12-04,2024-11-25,Split,,,,4-for-1,,4:1 split,
FFF,2024-12-20,2024-12-24,Buy,7,71.805,0.45,,USD,Broker margin acct-001,
DDD,2025-01-03,2025-01-06,Buy,1,48.9968,0.00,,USD,Broker margin acct-001,
EEE,2025-01-03,2025-01-06,Buy,6,59.0659,0.00,,USD,Broker margin acct-001,
CCC,2025-02-15,2025-02-15,Buy,115,106.87,0.00,,USD,ESPP,
CCC,2025-02-19,2025-02-20,Sell,19,104.704737,5.02,,USD,ESPP sell-to-cover,
",
        String::from_utf8(output).unwrap()
    );
}
