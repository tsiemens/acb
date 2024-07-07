use std::mem::swap;

use rust_decimal::{prelude::One, Decimal};
use time::Date;

use crate::{
    peripheral::sheet_common::SheetParseError,
    portfolio::{Affiliate, Currency, TxAction},
    util::decimal::is_positive,
};

use super::{Account, BrokerTx};

pub struct FxtRow {
    pub row_num: usize,
    pub currency: Currency,
    pub affiliate: Affiliate,
    pub trade_date: Date,
    pub trade_date_and_time: String,
    pub amount: Decimal,
    pub account: Account,
}

/// Tracks foreign-exchange transactions.
///
/// [!!!] TODO neither do we do accumulation, nor do we need to use integer values anymore
///
/// NOTE if we accumulate, the trade and settlement dates would need to be
/// the same for all accumulated transactions.
///
/// FXT transactions for each day are accumulated, and converted to a single
/// transaction purchasing that currency as an artificial security (the quantity
/// is inflated so that it can become an integer).
///
/// Sells and Buys in that currency also affect the daily transaction.
pub struct FxTracker {
    adjacent_fxt: Option<FxtRow>,

    // Generated BrokerTxs over the course of the tracking session
    txs: Vec<BrokerTx>,
}

impl FxTracker {
    pub fn new() -> Self {
        Self {
            adjacent_fxt: None,
            txs: Vec::new(),
        }
    }

    /// To be called with explicit foregn exchange transactions exported
    /// from the broker.
    /// These must always be performed in pairs, so add_fxt_row must
    /// be called twice for each conversion - Once for CAD and once for
    /// the other currency.
    pub fn add_fxt_row(&mut self, fxt_row: FxtRow) -> Result<(), SheetParseError> {
        if self.adjacent_fxt.is_none() {
            self.adjacent_fxt = Some(fxt_row);
            return Ok(());
        }

        // Swap these here, so that a failure doesn't cause us to re-use
        // the adjacent_fxt
        let mut adj_fx_opt: Option<FxtRow> = None;
        swap(&mut adj_fx_opt, &mut self.adjacent_fxt);
        let adj_fxt = adj_fx_opt.as_ref().unwrap();

        let (cad_fxt, other_fxt): (&FxtRow, &FxtRow) =
            if adj_fxt.currency == Currency::cad() {
                (adj_fxt, &fxt_row)
            } else {
                (&fxt_row, adj_fxt)
            };

        if cad_fxt.currency != Currency::cad()
            || other_fxt.currency == Currency::cad()
        {
            return Err(SheetParseError::new(
                fxt_row.row_num,
                format!(
                    "FXTs not supported between {} and \
                    {}. Exactly one currency must be CAD.",
                    cad_fxt.currency, other_fxt.currency
                ),
            ));
        }

        if other_fxt.currency == Currency::cad() {
            return Err(SheetParseError::new(
                fxt_row.row_num,
                format!(
                    "FXT on {} adjacent to CAD row was also CAD",
                    other_fxt.trade_date
                ),
            ));
        }
        if other_fxt.trade_date != cad_fxt.trade_date {
            return Err(SheetParseError::new(
                fxt_row.row_num,
                format!(
                    "Adjacent FXT rows on {} and {} were on different dates",
                    other_fxt.trade_date, cad_fxt.trade_date
                ),
            ));
        }
        if other_fxt.affiliate != cad_fxt.affiliate
            || other_fxt.account != cad_fxt.account
        {
            return Err(SheetParseError::new(
                fxt_row.row_num,
                format!(
                    "adjacent FXT rows on {} were in different accounts",
                    other_fxt.trade_date
                ),
            ));
        }
        if (cad_fxt.amount * other_fxt.amount) > Decimal::ZERO {
            return Err(if cad_fxt.amount > Decimal::ZERO {
                SheetParseError::new(
                    fxt_row.row_num,
                    String::from("Both FXTs have positive amounts"),
                )
            } else {
                SheetParseError::new(
                    fxt_row.row_num,
                    String::from("Both FXTs have negative amounts"),
                )
            });
        }

        let rate = (cad_fxt.amount / other_fxt.amount).abs();

        let tx = FxTracker::fx_tx(
            other_fxt.currency.clone(),
            other_fxt.trade_date.clone(),
            other_fxt.trade_date_and_time.clone(),
            other_fxt.amount,
            other_fxt.affiliate.clone(),
            fxt_row.row_num as usize,
            other_fxt.account.clone(),
            Some(rate),
            String::from("FXT"),
        )?;

        self.txs.push(tx);

        Ok(())
    }

    /// To be called with a "regular" tx (buy/sell) from the broker export,
    /// any time the currency is not CAD.
    /// This will add an inverse transaction of that currency.
    pub fn add_implicit_fxt(
        &mut self,
        tx: &BrokerTx,
    ) -> Result<(), SheetParseError> {
        let mut amount = tx.amount_per_share * tx.num_shares;
        if tx.action == TxAction::Buy {
            // We're buying some stock with USD, so our reserve will decrease.
            amount *= Decimal::NEGATIVE_ONE;
        }

        amount -= tx.commission;

        if amount.is_zero() {
            // This might happen for DIS actions, which get treated otherwise like
            // a buy.
            return Ok(());
        }

        let tx = FxTracker::fx_tx(
            tx.currency.clone(),
            tx.trade_date.clone(),
            tx.trade_date_and_time.clone(),
            amount,
            tx.affiliate.clone(),
            tx.row_num as usize,
            tx.account.clone(),
            None,
            format!("from {} {}", tx.security, tx.action.to_string()),
        )?;

        self.txs.push(tx);
        Ok(())
    }

    /// tx must be generated from fx_tx. Used for dividend payments, for eg.
    pub fn add_income_fx_tx(&mut self, tx: BrokerTx) {
        self.txs.push(tx)
    }

    pub fn get_fx_txs(
        &self,
    ) -> Result<&Vec<BrokerTx>, (&Vec<BrokerTx>, SheetParseError)> {
        if let Some(adj_fxt) = &self.adjacent_fxt {
            return Err((
                &self.txs,
                SheetParseError::new(adj_fxt.row_num, "Unpaired FXT".to_string()),
            ));
        }

        Ok(&self.txs)
    }

    pub fn fx_tx(
        currency: Currency,
        trade_date: Date,
        trade_date_and_time: String,
        amount: Decimal,
        affiliate: Affiliate,
        row_num: usize,
        account: Account,
        exchange_rate: Option<Decimal>,
        memo_extra: String,
    ) -> Result<BrokerTx, SheetParseError> {
        let action = if is_positive(&amount) {
            TxAction::Buy
        } else {
            TxAction::Sell
        };
        let (shares, amount_per_share) = match &currency {
            c if c == &Currency::usd() => (amount.abs(), Decimal::one()),
            _ => {
                return Err(SheetParseError::new(
                    row_num,
                    format!("FX currency {currency} not supported"),
                ));
            }
        };

        let security = currency.as_str().to_string() + ".FX";
        let memo = format!("{}; {}", account.memo_str(), memo_extra);

        Ok(BrokerTx {
            security: security,
            trade_date: trade_date,
            settlement_date: trade_date,
            trade_date_and_time: trade_date_and_time.clone(),
            // FXTs always settle immediately, and must sort by trade date anyway,
            // else we'll have incorrect balances.
            settlement_date_and_time: trade_date_and_time.clone(),
            action: action,
            amount_per_share: amount_per_share,
            num_shares: shares,
            commission: Decimal::ZERO,
            currency: currency,
            memo: memo,
            exchange_rate: exchange_rate,
            affiliate: affiliate,
            row_num: row_num as u32,
            account: account,
            // We need all FX to buy first, then sell. Otherwise we have quantity issues
            sort_tiebreak: Some(if action == TxAction::Buy { 1 } else { 2 }),
            filename: None,
        })
    }
}
