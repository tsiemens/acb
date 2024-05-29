use std::{rc::Rc, str::FromStr};

use acb::{
    app::outfmt::{model::AcbWriter, text::TextWriter},
    gezdec,
    portfolio::{
        bookkeeping::calc_total_costs,
        render::{render_total_costs, RenderTable},
        Affiliate, CurrencyAndExchangeRate, PortfolioSecurityStatus, RocTxSpecifics, Tx, TxActionSpecifics, TxDelta
    },
    util::{date::parse_standard_date, decimal::GreaterEqualZeroDecimal, rw::WriteHandle}
};
use rust_decimal::Decimal;
use time::Date;

#[derive(Debug)]
struct Td {
    pub sec: &'static str,
    pub settle: &'static str,
    pub total_acb: &'static str,
    pub affiliate: &'static str,
}

impl Td {
    pub fn settle_date(&self) -> Date {
        parse_standard_date(&self.settle).unwrap()
    }
}

fn get_delta(td: Td) -> TxDelta {
    let acb = if td.total_acb != "" {
        Some(GreaterEqualZeroDecimal::try_from(
            Decimal::from_str(td.total_acb).unwrap()).unwrap())
    } else { None };

    TxDelta {
        tx: Tx{
            security: td.sec.to_string(),
            trade_date: parse_standard_date("1970-01-01").unwrap(),
            settlement_date: td.settle_date(),
            // This is meaningless here. Just the simplest to populate
            action_specifics: TxActionSpecifics::Roc(RocTxSpecifics{
                amount_per_held_share: gezdec!(0),
                tx_currency_and_rate: CurrencyAndExchangeRate::default() }),
            memo: String::new(),
            affiliate: Affiliate::from_strep(&td.affiliate),
            read_index: 0,
        },
        pre_status: Rc::new(PortfolioSecurityStatus{
            security: td.sec.to_string(),
            total_acb: Some(gezdec!(0)),
            share_balance: gezdec!(0),
            all_affiliate_share_balance: gezdec!(0)}),
        post_status: Rc::new(PortfolioSecurityStatus{
            security: td.sec.to_string(),
            total_acb: acb,
            share_balance: gezdec!(0),
            all_affiliate_share_balance: gezdec!(0)}),
        capital_gain: None,
        sfl: None,
    }
}

fn get_deltas(tds: Vec<Td>) -> Vec<TxDelta> {
    tds.into_iter().map(|d| get_delta(d)).collect()
}

fn render_table(rt: &RenderTable) -> String {
    let (wh, buff) = WriteHandle::string_buff_write_handle();
    let mut wr = TextWriter::new(wh);
    wr.print_render_table(acb::app::outfmt::model::OutputType::Costs,
                          "Table of", &rt).unwrap();
    let buff_ref = buff.borrow();
    buff_ref.as_str().to_string()
}

/// Updates the (assumed decimal) values in `v` to be of the format $x.yy
fn fixup_rows(col_start_idx: usize, v: &mut Vec<Vec<String>>) {
    for i in 0..v.len() {
        let row_len = v.get(i).unwrap().len();
        for j in col_start_idx..row_len {
            let c = v.get(i).unwrap().get(j).unwrap().trim();
            v[i][j] = format!("${:.2}", Decimal::from_str(c).unwrap());
        }
    }
}

fn assert_tables_equal(fix_col_offset: usize,
                       mut expected: RenderTable, mut actual: RenderTable) {
    fixup_rows(fix_col_offset, &mut expected.rows);
    expected.notes.sort();
    actual.notes.sort();
    let exp_str = render_table(&expected);
    let act_str = render_table(&actual);
    tracing::debug!("Actual:\n{act_str}");
    if exp_str != act_str {
        eprintln!("Actual:\n{act_str}\n != expected:\n{exp_str}");
        assert_eq!(exp_str, act_str);
    }
}

fn run_test<T>(name: &str, test: T)
where
    T: FnOnce() + std::panic::UnwindSafe,
{
    println!("Running test: {}", name);
    let result = std::panic::catch_unwind(test);
    match result {
        Ok(_) => println!("{name} passed"),
        Err(e) => {
            panic!("{name} failed: {e:#?}");
        },
    }
}

#[test]
fn test_render_total_costs() {
    acb::tracing::setup_tracing();

    struct Case {
        pub name: &'static str,
        pub reorg: fn(&mut Vec<Td>),
    }

    for tc in vec![
        Case { name: "none", reorg: |_| {} },
        Case { name: "by-security", reorg: |data: &mut Vec<Td>| {
            data.sort_by(|a, b| {
                if a.sec != b.sec {
                    a.sec.partial_cmp(b.sec).unwrap()
                } else {
                    // We must always have Txs from the same security
                    // sorted by date.
                    a.settle_date().partial_cmp(&b.settle_date()).unwrap()
                }
            });
        } },
    ] {
        run_test(tc.name, || {
            let mut data = vec![
                Td{sec: "SECA", settle: "2001-01-13", total_acb: "100", affiliate: ""},
                Td{sec: "XXXX", settle: "2001-02-14", total_acb: "90", affiliate: ""},
                Td{sec: "SECA", settle: "2001-03-15", total_acb: "0", affiliate: ""},
                Td{sec: "XXXX", settle: "2001-04-16", total_acb: "80", affiliate: ""},
                Td{sec: "SECA", settle: "2001-05-17", total_acb: "200", affiliate: ""},
                Td{sec: "XXXX", settle: "2001-05-17", total_acb: "70", affiliate: ""},
                Td{sec: "SECA", settle: "2003-01-01", total_acb: "0", affiliate: ""},
                Td{sec: "SECA", settle: "2003-01-02", total_acb: "150", affiliate: ""},
                Td{sec: "XXXX", settle: "2003-01-02", total_acb: "35", affiliate: ""},
                Td{sec: "TFSA", settle: "2003-01-02", total_acb: "", affiliate: ""},
                Td{sec: "SPSE", settle: "2003-01-02", total_acb: "100", affiliate: "Spouse"},
                Td{sec: "SECA", settle: "2003-01-03", total_acb: "0", affiliate: ""},
            ];
            (tc.reorg)(&mut data);

            for d in &data {
                tracing::debug!("{d:#?}");
            }

            let all_deltas = get_deltas(data);
            let costs = calc_total_costs(&all_deltas);
            let costs_tables = render_total_costs(&costs, false);

            let notes = vec![
                "2003-01-02 (TFSA) ignored transaction from registered affiliate".to_string(),
                "2003-01-02 (SPSE) ignored transaction from non-default affiliate Spouse".to_string(),
            ];

            let svec = |v: Vec<&str>| -> Vec<String> {
                v.iter().map(|s| s.to_string()).collect()
            };

            let exp = RenderTable{
                header: svec(vec!["Date", "Total", "SECA", "XXXX"]),
                rows: vec![
                    svec(vec!["2001-01-13", "100", "100", " 0"]),
                    svec(vec!["2001-02-14", "190", "100", "90"]),
                    svec(vec!["2001-03-15", " 90", "  0", "90"]),
                    svec(vec!["2001-04-16", " 80", "  0", "80"]),
                    svec(vec!["2001-05-17", "270", "200", "70"]),
                    svec(vec!["2003-01-01", " 70", "  0", "70"]),
                    svec(vec!["2003-01-02", "185", "150", "35"]),
                    svec(vec!["2003-01-03", " 35", "  0", "35"]),
                ],
                footer: vec![],
                notes: notes.clone(),
                errors: vec![],
            };

            assert_tables_equal(1, exp, costs_tables.total);

            let exp_year = RenderTable{
                header: svec(vec!["Year", "Date", "Total", "SECA", "XXXX"]),
                rows: vec![
                    svec(vec!["2001", "2001-05-17", "270", "200", "70"]),
                    svec(vec!["2003", "2003-01-02", "185", "150", "35"]),
                ],
                footer: vec![],
                notes: notes.clone(),
                errors: vec![],
            };

            assert_tables_equal(2, exp_year, costs_tables.yearly);
        });
    }
}