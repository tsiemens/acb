use rust_decimal::Decimal;

use crate::util::decimal::{dollar_precision_str, is_negative, is_positive};

use super::{bookkeeping::Costs, CumulativeCapitalGains, CurrencyAndExchangeRate, TxDelta};

pub struct RenderTable {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub footer: Vec<String>,
    pub notes: Vec<String>,
    pub errors: Vec<String>,
}

pub struct CostsTables {
	pub total:  RenderTable,
	pub yearly: RenderTable,
}

struct PrintHelper {
    print_all_decimals: bool,
    display_none_env_setting: bool,
}

impl PrintHelper {
    pub fn new(print_all_decimals: bool) -> PrintHelper {
        // Set our setting to explicitly display None instead
        // of just '-'. This can make things slightly more clear
        // while debugging.
        let display_none_env_setting = match std::env::var("DISPLAY_OPT_NONE") {
            Ok(value) => {
                !(value.trim().is_empty() || value.trim() == "0")
            },
            Err(_) => false,
        };

        PrintHelper{ print_all_decimals, display_none_env_setting }
    }

    /// Gets the pretty string representation we will
    /// use to render Option::None.
    /// Users most of the time do not care about the implementation
    /// detail, so we don't necessarily want to show "None",
    /// but we may in some cases for development.
    fn str_for_none(&self) -> &str {
        if self.display_none_env_setting { "None" } else { "-" }
    }

    /// Get the representation for a currency *value*, without
    /// any indication of what that currency is (like a dollar sign).
    fn curr_str(&self, val: Decimal) -> String {
        if self.print_all_decimals {
            val.to_string()
        } else {
            dollar_precision_str(&val)
        }
    }

    /// Get a pretty representation for a dollar value
    pub fn dollar_str(&self, val: Decimal) -> String {
        format!("${}", self.curr_str(val))
    }

    pub fn opt_dollar_str(&self, opt_val: Option<Decimal>) -> String {
        match opt_val {
            Some(val) => self.dollar_str(val),
            None => self.str_for_none().to_string(),
        }
    }

    /// Get a pretty representation for a currency value, which
    /// could be either in the default or a non-default currency.
    pub fn curr_with_fx_str(&self,
        val: Decimal, fx: &CurrencyAndExchangeRate) -> String {
        if fx.currency.is_default() {
            self.dollar_str(val)
        } else {
            let local_val = val * *fx.exchange_rate;
            format!("{}\n({} {})", self.dollar_str(local_val), self.curr_str(val), fx.currency)
        }
    }

    pub fn plus_minus_dollar(&self, val: Decimal, show_plus: bool) -> String {
        self.plus_minus_opt_dollar(Some(val), show_plus)
    }

    pub fn plus_minus_opt_dollar(&self, opt_val: Option<Decimal>, show_plus: bool) -> String {
        let val = match opt_val {
            Some(v) => v,
            None => { return self.str_for_none().to_string(); },
        };
        if is_negative(&val) {
            format!("-${}", self.curr_str(val * Decimal::NEGATIVE_ONE))
        } else {
            let plus = if show_plus { "+" } else { "" };
            format!("{}${}", plus, self.curr_str(val))
        }
    }
}

/// For any string s, splits it so that it will fit into a column
/// of width `max_width`.
/// Newlines are respected. Only breaks across words, so if there
/// are any words longer than max_width, they will force the
/// line-width to exceed the max.
fn wrap_str_to_width(s: &str, max_width: usize) -> String {
    let mut new_lines = Vec::<String>::new();
    for line in s.split("\n") {
        if line.len() > max_width {
            let mut new_line = String::new();
            let words = line.split(" ");
            for word in words {
                if new_line.is_empty() {
                    new_line += word;
                } else {
                    if (new_line.len() + 1 + word.len()) > max_width {
                        // We need to wrap here.
                        new_lines.push(new_line);
                        new_line = word.to_string();
                    } else {
                        new_line += " ";
                        new_line += word;
                    }
                }
            }

            if !new_line.is_empty() {
                new_lines.push(new_line);
            }
        } else {
            new_lines.push(line.to_string());
        }
    }

    new_lines.join("\n")
}

pub fn render_tx_table_model(
	deltas: &Vec<TxDelta>, gains: &CumulativeCapitalGains,
    render_full_dollar_values: bool
) -> RenderTable {

    let s = String::from;
    let ph = PrintHelper::new(render_full_dollar_values);

    let header: Vec<String> = vec![
        "Security", "Trade Date", "Settl. Date", "TX", "Amount", "Shares", "Amt/Share", "ACB",
		"Commission", "Cap. Gain", "Share Balance", "ACB +/-", "New ACB", "New ACB/Share",
		"Affiliate", "Memo",
    ].iter().map(|s| s.to_string()).collect();

    let mut rows = Vec::<Vec<String>>::new();

    let mut saw_superficial_loss = false;
    let mut saw_over_applied_sfl = false;

    for d in deltas {
        let mut superficial_loss_asterix = "".to_string();
        let tx = &d.tx;

        match &tx.action_specifics {
            super::TxActionSpecifics::Sell(sell_specs) => {
                let specified_sfl_is_forced = if let Some(sfl) = &sell_specs.specified_superficial_loss {
                    sfl.force } else { false };
                if d.is_superficial_loss() {
                    let sfl = d.sfl.as_ref().unwrap();
                    let extra_sfl_note_str = if sfl.potentially_over_applied {
                        "[1]" } else { "" };

                    superficial_loss_asterix = format!(
                        " *\n(SfL {}{}; {}{}",
                        ph.plus_minus_dollar(*sfl.superficial_loss, false),
                        if specified_sfl_is_forced { "!" } else { "" },
                        sfl.ratio,
                        extra_sfl_note_str
                    );
                    saw_superficial_loss = true;
                    saw_over_applied_sfl |= sfl.potentially_over_applied;
                }
            },
            _ => {},
        }

        let buy_sell_specs = match &tx.action_specifics {
            super::TxActionSpecifics::Sell(specs) => Some(specs.common_buy_sell_attrs()),
            super::TxActionSpecifics::Buy(specs) => Some(specs.common_buy_sell_attrs()),
            _ => None,
        };



        let mut amount = String::new();
        let mut shares: Decimal = Decimal::ZERO;
        let mut amount_per_share_str = String::new();
        let mut acb_of_sale: Option<String> = None;
        let mut commission: Option<String> = None;
        let mut capital_gains: Option<String> = None;
        let mut new_share_balance = String::new();

        if let Some(buy_sell_specs) = buy_sell_specs {
            amount = ph.curr_with_fx_str(*buy_sell_specs.shares * *buy_sell_specs.amount_per_share,
                                         &buy_sell_specs.tx_currency_and_rate);
            shares = *buy_sell_specs.shares;
            amount_per_share_str = ph.curr_with_fx_str(
                *buy_sell_specs.amount_per_share, &buy_sell_specs.tx_currency_and_rate);
            if tx.action() == super::TxAction::Sell {
                acb_of_sale = if is_positive(&*d.pre_status.share_balance) {
                    match d.pre_status.total_acb {
                        Some(pre_total_acb) => {
                            let pre_acb_per_share = *pre_total_acb / *d.pre_status.share_balance;
                            Some(ph.dollar_str(pre_acb_per_share * *buy_sell_specs.shares))
                        },
                        None => None,
                    }
                } else { None };

                if let Some(cap_gain) = d.capital_gain {
                    capital_gains = Some(format!("{}{}", ph.plus_minus_dollar(cap_gain, false),
                                         superficial_loss_asterix));
                }
            }

            commission = if buy_sell_specs.commission.is_zero() { None } else {
                Some(ph.curr_with_fx_str(*buy_sell_specs.commission,
                                         buy_sell_specs.commission_currency_and_rate()))
            };

            new_share_balance = if d.post_status.share_balance == d.post_status.all_affiliate_share_balance {
                d.post_status.share_balance.to_string()
            } else {
                format!("{} / {}", d.post_status.share_balance, d.post_status.all_affiliate_share_balance)
            }
        } else if let super::TxActionSpecifics::Roc(roc_specs) = &tx.action_specifics {
            shares = *d.pre_status.share_balance;
            amount = ph.curr_with_fx_str(shares * *roc_specs.amount_per_held_share,
                &roc_specs.tx_currency_and_rate);
            amount_per_share_str = ph.curr_with_fx_str(
              *roc_specs.amount_per_held_share, &roc_specs.tx_currency_and_rate);

        } else if let super::TxActionSpecifics::Sfla(sfla_specs) = &tx.action_specifics {
            amount = ph.curr_with_fx_str(*sfla_specs.shares_affected * *sfla_specs.amount_per_share,
                                         &CurrencyAndExchangeRate::default());
            shares = *sfla_specs.shares_affected;
            amount_per_share_str = ph.curr_with_fx_str(
                *sfla_specs.amount_per_share, &CurrencyAndExchangeRate::default());
        }

        let acb_per_share: Option<String> =
            if is_positive(&*d.post_status.share_balance) {
                if let Some(post_acb) = d.post_status.total_acb {
                    Some(ph.dollar_str(*post_acb / *d.post_status.share_balance))
                } else {
                    None
                }
            } else {
                None
            };

        let row = vec![
            tx.security.clone(),
            tx.trade_date.to_string(),
            tx.settlement_date.to_string(),
            tx.action().pretty_str().to_string(),
            amount,
            shares.to_string(),
            amount_per_share_str,
            acb_of_sale.unwrap_or(s("-")),
            commission.unwrap_or(s("-")),
            capital_gains.unwrap_or(s("-")),
            new_share_balance,
            ph.plus_minus_opt_dollar(d.acb_delta(), true),
            ph.opt_dollar_str(d.post_status.total_acb.map(|d| *d)),
            acb_per_share.unwrap_or(s("-")),
            tx.affiliate.name().to_string(),
            wrap_str_to_width(tx.memo.as_str(), 32),
        ];
        rows.push(row);
    } // for d in deltas

    // Footer
    let years = gains.capital_gains_year_totals_keys_sorted();
    let mut year_strs = Vec::<String>::new();
    let mut year_val_strs = Vec::<String>::new();
    for year in &years {
        year_strs.push(year.to_string());
        let yearly_total = *gains.capital_gains_years_totals.get(&year).unwrap();
        year_val_strs.push(ph.plus_minus_dollar(yearly_total, false));
    }
    let mut total_footer_label = s("Total");
    let mut total_footer_vals_str = ph.plus_minus_dollar(gains.capital_gains_total, false);
    if years.len() > 0 {
        total_footer_label = format!("{}{}{}", total_footer_label, "\n", year_strs.join("\n"));
        total_footer_vals_str = format!("{}{}{}", total_footer_vals_str, "\n", year_val_strs.join("\n"));
    }

    let mut footer = Vec::with_capacity(header.len());
    footer.resize(header.len(), s(""));
    footer[8] = total_footer_label;
    footer[9] = total_footer_vals_str;

    // Notes
    let mut notes = Vec::<String>::new();
    if saw_superficial_loss {
        notes.push(s(" SfL = Superficial loss adjustment"));
    }
    if saw_over_applied_sfl {
        notes.push(s(" [1] Superficial loss was potentially over-applied, \
        resulting in a lower-than-expected allowable capital loss.\n     \
             See I.1 vs I.2 under \"Interpretations of ACB distribution\" at \
             https://github.com/tsiemens/acb/wiki/Superficial-Losses"));
    }

    RenderTable {
        header,
        rows,
        footer: footer,
        notes: notes,
        errors: Vec::new(),
    }
}

/// RenderAggregateCapitalGains generates a RenderTable that will render out to this:
///
///	| Year             | Capital Gains |
///	+------------------+---------------+
///	| 2000             | xxxx.xx       |
///	| 2001             | xxxx.xx       |
///	| Since inception  | xxxx.xx       |
pub fn render_aggregate_capital_gains(
    gains: &CumulativeCapitalGains, render_full_dollar_values: bool
) -> RenderTable {
    let s = String::from;

    let ph = PrintHelper::new(render_full_dollar_values);

    let mut rows = Vec::<Vec<String>>::new();

    let years = gains.capital_gains_year_totals_keys_sorted();
    for year in years {
        let yearly_total = gains.capital_gains_years_totals[&year];

        rows.push(
            vec![
                year.to_string(),
                ph.plus_minus_dollar(yearly_total, false),
            ]
        );
    }

    RenderTable{
        header: vec![s("Year"), s("Capital Gains")],
        rows: rows,
        footer: vec![],
        notes: vec![],
        errors: vec![],
    }
}

pub fn render_total_costs(costs: &Costs, render_full_dollar_values: bool) -> CostsTables {
    let mut securities: Vec<&String> = costs.security_set.iter().collect();
    securities.sort();

    let s = String::from;

    let ph = PrintHelper::new(render_full_dollar_values);

    // Total table:

    let mut total_costs_headers = Vec::<String>::with_capacity(2 + securities.len());
    total_costs_headers.push(s("Date"));
    total_costs_headers.push(s("Total"));
    total_costs_headers.append(&mut securities.iter().map(|s| (**s).clone()).collect());

    let mut total_costs_rows = Vec::<Vec<String>>::new();
    for max_day_costs in &costs.total {
        // Row is: Date, Total, Securities...
        let mut row = Vec::<String>::with_capacity(2 + securities.len());
        row.push(max_day_costs.day.to_string());
        row.push(ph.dollar_str(*max_day_costs.total));
        for sec in &securities {
            row.push(ph.dollar_str(
                **max_day_costs.sec_max_cost_for_day.get(*sec).unwrap()))
        }

        total_costs_rows.push(row);
    }

    let total_table = RenderTable {
        header: total_costs_headers,
        rows: total_costs_rows,
        footer: vec![],
        notes: costs.ignored_deltas.clone(),
        errors: vec![],
    };

    // Yearly table:

    let mut yearly_costs_headers = Vec::<String>::with_capacity(3 + securities.len());
    yearly_costs_headers.push(s("Year"));
    yearly_costs_headers.push(s("Date"));
    yearly_costs_headers.push(s("Total"));
    yearly_costs_headers.append(&mut securities.iter().map(|s| (**s).clone()).collect());

    let mut year_costs_rows = Vec::<Vec<String>>::new();

    for year in costs.sorted_years() {
        // Row is: Year, Date, Total, Securities...
        let max_year_costs = costs.yearly.get(&year).unwrap();

        let mut row = Vec::<String>::with_capacity(3 + securities.len());
        row.push(year.to_string());
        row.push(max_year_costs.day.to_string());
        row.push(ph.dollar_str(*max_year_costs.total));
        for sec in &securities {
            row.push(ph.dollar_str(
                **max_year_costs.sec_max_cost_for_day.get(*sec).unwrap()))
        }

        year_costs_rows.push(row);
    }

    let yearly_table = RenderTable {
        header: yearly_costs_headers,
        rows: year_costs_rows,
        footer: vec![],
        notes: costs.ignored_deltas.clone(),
        errors: vec![],
    };

    CostsTables {
        total: total_table,
        yearly: yearly_table,
    }
}

// MARK: Tests
#[cfg(test)]
mod tests {
    use crate::portfolio::render::wrap_str_to_width;

    #[test]
    fn test_wrap_str_to_width() {
        assert_eq!(wrap_str_to_width("", 10).as_str(), "");
        assert_eq!(wrap_str_to_width("Bla", 10).as_str(), "Bla");
        assert_eq!(wrap_str_to_width("  Bla  ", 10).as_str(), "  Bla  ");
        assert_eq!(wrap_str_to_width("Bla\n  ", 10).as_str(), "Bla\n  ");
        assert_eq!(wrap_str_to_width("Bla\nbla", 10).as_str(), "Bla\nbla");

        assert_eq!(wrap_str_to_width("Verylongword\nbla", 5).as_str(), "Verylongword\nbla");
        assert_eq!(wrap_str_to_width("Verylongword bla", 5).as_str(), "Verylongword\nbla");
        assert_eq!(wrap_str_to_width("Bla Verylongword", 5).as_str(), "Bla\nVerylongword");
        assert_eq!(wrap_str_to_width("Bla1 bla2 bla3 bla4", 10).as_str(), "Bla1 bla2\nbla3 bla4");

    }
}