use std::fmt::Display;

use rust_decimal::Decimal;
use time::Date;

use crate::{
    portfolio::{csv_common::CsvCol, Affiliate},
    util::decimal::{GreaterEqualZeroDecimal, LessEqualZeroDecimal, PosDecimal},
};

use super::currency::{Currency, CurrencyAndExchangeRate};

pub type Security = String;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TxAction {
    Buy,
    Sell,
    Roc,   // Return of capital
    Sfla,  // Superficial loss ACB adjustment
    Split, // Stock split/reverse-split
}

impl TxAction {
    pub fn pretty_str(&self) -> &str {
        match self {
            TxAction::Buy => "Buy",
            TxAction::Sell => "Sell",
            TxAction::Roc => "RoC",
            TxAction::Sfla => "SfLA",
            TxAction::Split => "Split",
        }
    }
}

impl TryFrom<&str> for TxAction {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let re_ci_match = |s| -> bool {
            regex::RegexBuilder::new(s)
                .case_insensitive(true)
                .build()
                .unwrap()
                .is_match(value)
        };

        // Finds the word within the value, since in some cases (namely, Etrade
        // will sometimes mark sales as "Sold Short") there are extra words included
        // in the action description. It doesn't really make a difference for us
        // though (at least not yet).
        if re_ci_match(r"\b(buy|bought)\b") {
            Ok(TxAction::Buy)
        } else if re_ci_match(r"\b(sell|sold)\b") {
            Ok(TxAction::Sell)
        } else if re_ci_match(r"\b(roc)\b") {
            Ok(TxAction::Roc)
        } else if re_ci_match(r"\b(sfla)\b") {
            Ok(TxAction::Sfla)
        } else if re_ci_match(r"\b(split)\b") {
            Ok(TxAction::Split)
        } else {
            Err(format!("Unable to parse action from '{value}'"))
        }
    }
}

impl Display for TxAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pretty_str())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SFLInput {
    pub superficial_loss: LessEqualZeroDecimal,
    pub force: bool,
}

impl SFLInput {
    pub fn req_from_dec(v: Decimal, force: bool) -> SFLInput {
        SFLInput {
            superficial_loss: LessEqualZeroDecimal::try_from(v).unwrap(),
            force: force,
        }
    }
}

/// Ratio for a stock split. Can be a split or reverse split.
///
/// If reverse_integer_only is true (false if parsed string has any decimal
/// precision) then a reverse split will enforce that there is no remainder stock
/// after the split. eg. If we have 5 shares, and do a 1-for-2 split, a failure will
/// occur unless we add a predecing sale of 1 share, or specify as a 1.0-for-2.0,
/// in which case, we'll be left with 2.5 shares.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SplitRatio {
    pub pre_split: PosDecimal,
    pub post_split: PosDecimal,
    pub reverse_integer_only: bool,
}

impl SplitRatio {
    /// Supports this fairly standard format: N-for-M or Post-for-Pre
    /// This makes the direction of the split very clear.
    /// Technically can support non-integer ratios.
    pub fn parse(str_repr: &str) -> Result<Self, crate::util::basic::SError> {
        let m = regex::RegexBuilder::new(r"^\s*([\d\.]+)-for-([\d\.]+)\s*$")
            .case_insensitive(true)
            .build()
            .unwrap()
            .captures(str_repr);
        if let Some(caps) = m {
            let post_str = caps.get(1).unwrap().as_str();
            let post = PosDecimal::try_from(
                Decimal::from_str_exact(post_str).map_err(|e| e.to_string())?,
            )?;
            let pre_str = caps.get(2).unwrap().as_str();
            let pre = PosDecimal::try_from(
                Decimal::from_str_exact(pre_str).map_err(|e| e.to_string())?,
            )?;

            let decimal_re = regex::Regex::new(r"\.\d").unwrap();
            let reverse_integer_only =
                !decimal_re.is_match(post_str) && !decimal_re.is_match(pre_str);

            let mut ratio = SplitRatio {
                pre_split: pre,
                post_split: post,
                reverse_integer_only: reverse_integer_only,
            };
            if !ratio.is_reverse_split() {
                ratio.reverse_integer_only = false;
            }
            Ok(ratio)
        } else {
            Err(format!(
                "\"{}\" does not match N-for-M split format",
                str_repr
            ))
        }
    }

    pub fn is_reverse_split(&self) -> bool {
        *self.pre_split > *self.post_split
    }

    pub fn pre_to_post_factor(&self) -> PosDecimal {
        self.post_split / self.pre_split
    }
}

impl Display for SplitRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(precision) = f.precision() {
            write!(
                f,
                "{:.2$}-for-{:.2$}",
                self.post_split, self.pre_split, precision
            )
        } else if self.post_split.is_integer() && self.pre_split.is_integer() {
            if self.is_reverse_split() && !self.reverse_integer_only {
                // Write out with precision, so that re-parsing will set
                // reverse_integer_only to false again.
                write!(f, "{:.1}-for-{:.1}", self.post_split, self.pre_split)
            } else {
                write!(f, "{:.0}-for-{:.0}", self.post_split, self.pre_split)
            }
        } else {
            write!(f, "{}-for-{}", self.post_split, self.pre_split)
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TotalOrAmountPerShare<T> {
    Total(T),
    AmountPerShare(T),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SharesAndAmount<T> {
    pub shares: T,
    pub amount_per_share: T,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TotalAmount<T> {
    ExplicitTotal(T),
    TotalByShare(SharesAndAmount<T>),
}

impl <T> TotalAmount<T> {
    pub fn from_per_share(
        shares: T,
        amount_per_share: T,
    ) -> Self {
        TotalAmount::TotalByShare(SharesAndAmount {
            shares,
            amount_per_share,
        })
    }

    pub fn from_total(total: T) -> Self {
        TotalAmount::ExplicitTotal(total)
    }
}

impl TotalAmount<Decimal> {
    pub fn total_amount(&self) -> Decimal {
        match self {
            Self::ExplicitTotal(total) => *total,
            Self::TotalByShare(shares_and_amount) => {
                shares_and_amount.shares * shares_and_amount.amount_per_share
            }
        }
    }
}

impl TotalAmount<PosDecimal> {
    pub fn total_amount(&self) -> PosDecimal {
        match self {
            Self::ExplicitTotal(total) => *total,
            Self::TotalByShare(shares_and_amount) => {
                // Just unwrap here, since positive * positive is always positive.
                PosDecimal::try_from(*shares_and_amount.shares
                    * (*shares_and_amount.amount_per_share))
                    .unwrap()
            }
        }
    }
}

// This Transaction type is flat, and designed to absorb input from a uniform
// CSV/table input. It is fairly unconstrained, and as such, should be converted
// to the Tx type for processing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CsvTx {
    pub security: Option<Security>,
    pub trade_date: Option<Date>,
    pub settlement_date: Option<Date>,
    pub action: Option<TxAction>,
    pub shares: Option<Decimal>,
    // total_amount and amount_per_share are mutally exclusive
    pub amount_per_share: Option<Decimal>,
    pub total_amount: Option<Decimal>,
    pub commission: Option<Decimal>,

    pub tx_currency: Option<Currency>,
    pub tx_curr_to_local_exchange_rate: Option<Decimal>,

    pub commission_currency: Option<Currency>,
    pub commission_curr_to_local_exchange_rate: Option<Decimal>,

    pub memo: Option<String>,
    pub affiliate: Option<Affiliate>,

    // More commonly optional fields/columns

    // The total superficial loss for the transaction, as explicitly
    // specified by the user. May be cross-validated against calculated SFL to emit
    // warnings. If specified, the user is also required to specify one or more
    // SfLA Txs following this one, accounting for all shares experiencing the loss.
    // NOTE: This is always a negative (or zero) value in CAD, so that it matches the
    // displayed value
    pub specified_superficial_loss: Option<SFLInput>,

    pub stock_split_ratio: Option<SplitRatio>,

    // The absolute order in which the Tx was read from file or entered.
    // Used as a tiebreak in sorting.
    pub read_index: u32,
}

impl Default for CsvTx {
    fn default() -> Self {
        CsvTx {
            security: None,
            trade_date: None,
            settlement_date: None,
            action: None,
            shares: None,
            amount_per_share: None,
            total_amount: None,
            commission: None,
            tx_currency: None,
            tx_curr_to_local_exchange_rate: None,
            commission_currency: None,
            commission_curr_to_local_exchange_rate: None,
            memo: None,
            affiliate: None,
            specified_superficial_loss: None,
            stock_split_ratio: None,
            read_index: 0,
        }
    }
}

impl PartialOrd for CsvTx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let date_cmp = self.settlement_date.cmp(&other.settlement_date);
        match date_cmp {
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater => Some(date_cmp),
            std::cmp::Ordering::Equal => {
                Some(self.read_index.cmp(&other.read_index))
            }
        }
    }
}

impl Ord for CsvTx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl From<Tx> for CsvTx {
    fn from(value: Tx) -> Self {
        value.to_csvtx()
    }
}

impl CsvTx {
    pub fn get_opt_sane_total_or_amount_per_share(&self) -> Result<Option<TotalOrAmountPerShare<Decimal>>, String> {
        if self.amount_per_share.is_none() && self.total_amount.is_none() {
            return Ok(None);
        }
        if self.amount_per_share.is_some() && self.total_amount.is_some() {
            return Err(format!(
                "Both '{}' and '{}' were specified. Only one may be specified",
                CsvCol::AMOUNT_PER_SHARE, CsvCol::TOTAL_AMOUNT));
        }
        let total_or_amount = if let Some(total) = self.total_amount {
            TotalOrAmountPerShare::Total(total)
        } else {
            TotalOrAmountPerShare::AmountPerShare(self.amount_per_share.unwrap())
        };
        Ok(Some(total_or_amount))
    }

    pub fn get_sane_total_or_amount_per_share(&self) -> Result<TotalOrAmountPerShare<Decimal>, String> {
        self.get_opt_sane_total_or_amount_per_share()?.ok_or(
            format!("'{}' or '{}' not found",
                CsvCol::AMOUNT_PER_SHARE, CsvCol::TOTAL_AMOUNT))
    }

    pub fn get_sane_opt_total_amount(&self) -> Result<Option<TotalAmount<Decimal>>, String> {
        let total_or_amount = self.get_opt_sane_total_or_amount_per_share()?;
        let total: TotalAmount<Decimal> =
            match total_or_amount {
                Some(TotalOrAmountPerShare::Total(total)) => {
                    TotalAmount::ExplicitTotal(total)
                }
                Some(TotalOrAmountPerShare::AmountPerShare(amount_per_share)) => {
                    let shares = self.shares.ok_or(format!(
                        "Unable to compute total amount. '{}' specified, but '{}' not found",
                        CsvCol::AMOUNT_PER_SHARE, CsvCol::SHARES))?;
                    TotalAmount::from_per_share(shares, amount_per_share)
                }
                None => return Ok(None),
            };

        Ok(Some(total))
    }

    pub fn get_sane_total_amount(&self) -> Result<TotalAmount<Decimal>, String> {
        self.get_sane_opt_total_amount()?.ok_or(
            format!("'{}' or '{}' not found",
                CsvCol::AMOUNT_PER_SHARE, CsvCol::TOTAL_AMOUNT))
    }

    pub fn get_sane_opt_amount_per_share(&self) -> Result<Option<Decimal>, String> {
        let amount = self.get_opt_sane_total_or_amount_per_share()?;
        let amount_per_share: Option<Decimal> =
            match amount {
                Some(TotalOrAmountPerShare::Total(total)) => {
                    let shares = self.shares.ok_or(format!(
                        "Unable to compute amount per share. '{}' specified, but '{}' not found",
                        CsvCol::TOTAL_AMOUNT, CsvCol::SHARES))?;
                    if shares == Decimal::ZERO {
                        return Err(format!(
                            "Unable to compute amount per share. '{}' specified, but '{}' is zero",
                            CsvCol::TOTAL_AMOUNT, CsvCol::SHARES));
                    }
                    Some(total / shares)
                }
                Some(TotalOrAmountPerShare::AmountPerShare(amount_per_share)) => {
                    Some(amount_per_share)
                }
                None => None,
            };

        Ok(amount_per_share)
    }

    pub fn get_sane_amount_per_share(&self) -> Result<Decimal, String> {
        self.get_sane_opt_amount_per_share()?.ok_or(
            format!("'{}' or '{}' not found",
                CsvCol::AMOUNT_PER_SHARE, CsvCol::TOTAL_AMOUNT))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuyTxSpecifics {
    pub shares: PosDecimal,
    pub amount_per_share: GreaterEqualZeroDecimal,
    pub commission: GreaterEqualZeroDecimal,

    pub tx_currency_and_rate: CurrencyAndExchangeRate,
    pub separate_commission_currency: Option<CurrencyAndExchangeRate>,
}

type CommonBuySellAttrs = BuyTxSpecifics;

impl BuyTxSpecifics {
    pub fn from_common_buy_sell_attrs(common: CommonBuySellAttrs) -> Self {
        common
    }

    pub fn common_buy_sell_attrs(&self) -> CommonBuySellAttrs {
        self.clone()
    }

    pub fn commission_currency_and_rate(&self) -> &CurrencyAndExchangeRate {
        self.separate_commission_currency
            .as_ref()
            .unwrap_or(&self.tx_currency_and_rate)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SellTxSpecifics {
    pub shares: PosDecimal,
    pub amount_per_share: GreaterEqualZeroDecimal,
    pub commission: GreaterEqualZeroDecimal,

    pub tx_currency_and_rate: CurrencyAndExchangeRate,
    pub separate_commission_currency: Option<CurrencyAndExchangeRate>,

    pub specified_superficial_loss: Option<SFLInput>,
}

impl SellTxSpecifics {
    pub fn from_common_buy_sell_attrs(
        common: &CommonBuySellAttrs,
        specified_superficial_loss: Option<SFLInput>,
    ) -> Self {
        Self {
            shares: common.shares,
            amount_per_share: common.amount_per_share,
            commission: common.commission,
            tx_currency_and_rate: common.tx_currency_and_rate.clone(),
            separate_commission_currency: common
                .separate_commission_currency
                .clone(),
            specified_superficial_loss: specified_superficial_loss,
        }
    }

    pub fn common_buy_sell_attrs(&self) -> CommonBuySellAttrs {
        CommonBuySellAttrs {
            shares: self.shares,
            amount_per_share: self.amount_per_share,
            commission: self.commission,
            tx_currency_and_rate: self.tx_currency_and_rate.clone(),
            separate_commission_currency: self.separate_commission_currency.clone(),
        }
    }

    pub fn commission_currency_and_rate(&self) -> &CurrencyAndExchangeRate {
        self.separate_commission_currency
            .as_ref()
            .unwrap_or(&self.tx_currency_and_rate)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RocTxSpecifics {
    pub amount: TotalOrAmountPerShare<GreaterEqualZeroDecimal>,
    pub tx_currency_and_rate: CurrencyAndExchangeRate,
}

impl RocTxSpecifics {
    pub fn amount_per_held_share(&self) -> Option<GreaterEqualZeroDecimal> {
        match &self.amount {
            TotalOrAmountPerShare::Total(_) => None,
            TotalOrAmountPerShare::AmountPerShare(amount_per_share) => {
                Some(*amount_per_share)
            }
        }
    }

    pub fn total_amount(&self) -> Option<GreaterEqualZeroDecimal> {
        match &self.amount {
            TotalOrAmountPerShare::Total(total) => Some(*total),
            TotalOrAmountPerShare::AmountPerShare(_) => None,
        }
    }
}

type TotalPosAmount = TotalAmount<PosDecimal>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SflaTxSpecifics {
    _total_amount: TotalPosAmount,
}

impl SflaTxSpecifics {
    pub fn from_per_share(
        shares: PosDecimal,
        amount_per_share: PosDecimal,
    ) -> Self {
        SflaTxSpecifics {
            _total_amount: TotalPosAmount::from_per_share(shares, amount_per_share),
        }
    }

    pub fn from_total(
        total: PosDecimal,
    ) -> Self {
        SflaTxSpecifics {
            _total_amount: TotalPosAmount::from_total(total),
        }
    }

    pub fn total_amount(&self) -> PosDecimal {
        self._total_amount.total_amount()
    }

    pub fn explicit_total_amount(&self) -> Option<PosDecimal> {
        match &self._total_amount {
            TotalPosAmount::ExplicitTotal(total) => Some(*total),
            TotalPosAmount::TotalByShare(_) => None,
        }
    }

    pub fn shares_affected(&self) -> Option<PosDecimal> {
        match &self._total_amount {
            TotalPosAmount::TotalByShare(shares_and_amount) => {
                Some(shares_and_amount.shares)
            }
            TotalPosAmount::ExplicitTotal(_) => None,
        }
    }

    pub fn amount_per_share(&self) -> Option<PosDecimal> {
        match &self._total_amount {
            TotalPosAmount::TotalByShare(shares_and_amount) => {
                Some(shares_and_amount.amount_per_share)
            }
            TotalPosAmount::ExplicitTotal(_) => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SplitTxSpecifics {
    pub ratio: SplitRatio,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TxActionSpecifics {
    Buy(BuyTxSpecifics),
    Sell(SellTxSpecifics),
    Roc(RocTxSpecifics),
    Sfla(SflaTxSpecifics),
    Split(SplitTxSpecifics),
}

impl TxActionSpecifics {
    pub fn action(&self) -> TxAction {
        match self {
            TxActionSpecifics::Buy(_) => TxAction::Buy,
            TxActionSpecifics::Sell(_) => TxAction::Sell,
            TxActionSpecifics::Roc(_) => TxAction::Roc,
            TxActionSpecifics::Sfla(_) => TxAction::Sfla,
            TxActionSpecifics::Split(_) => TxAction::Split,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tx {
    pub security: Security,
    pub trade_date: Date,
    pub settlement_date: Date,
    pub action_specifics: TxActionSpecifics,
    pub memo: String,
    pub affiliate: Affiliate,
    pub read_index: u32,
}

impl Tx {
    pub fn action(&self) -> TxAction {
        self.action_specifics.action()
    }

    pub fn buy_specifics(&self) -> Result<&BuyTxSpecifics, ()> {
        match &self.action_specifics {
            TxActionSpecifics::Buy(specs) => Ok(specs),
            _ => Err(()),
        }
    }

    pub fn sell_specifics(&self) -> Result<&SellTxSpecifics, ()> {
        match &self.action_specifics {
            TxActionSpecifics::Sell(specs) => Ok(specs),
            _ => Err(()),
        }
    }

    pub fn to_csvtx(&self) -> CsvTx {
        let mut csvtx = CsvTx::default();

        populate_csvtx_fields_from_action_specifics(
            &self.action_specifics,
            &mut csvtx,
        );
        csvtx.security = Some(self.security.clone());
        csvtx.trade_date = Some(self.trade_date);
        csvtx.settlement_date = Some(self.settlement_date);
        csvtx.memo = Some(self.memo.clone());
        csvtx.affiliate = Some(self.affiliate.clone());
        csvtx.read_index = self.read_index;

        csvtx
    }
}

fn get_valid_exchange_rate(
    curr_col_name: &str,
    found_curr: &Option<Currency>,
    fx_col_name: &str,
    found_fx: &Option<Decimal>,
) -> Result<Option<CurrencyAndExchangeRate>, String> {
    if found_curr.is_none() && found_fx.is_none() {
        Ok(None)
    } else {
        let curr = found_curr.clone().ok_or(format!(
            "\"{fx_col_name}\" specified but \"{curr_col_name}\" not found"
        ))?;
        if curr.is_default() && found_fx.is_none() {
            // The rate here is implicitly 1, so the rate isn't required.
            // If rate is provided, we continue below to check its validity.
            return Ok(Some(CurrencyAndExchangeRate::default()));
        }
        let rate = found_fx.ok_or(format!(
            "\"{curr_col_name}\" specified but \"{fx_col_name}\" not found"
        ))?;
        Ok(Some(CurrencyAndExchangeRate::try_new(
            curr,
            PosDecimal::try_from(rate).map_err(|_| {
                format!("\"{fx_col_name}\" must be a positive value")
            })?,
        )?))
    }
}

// For now, just return BuyTxSpecifics, since it is a strict subset of
// SellTxSpecifics
fn buy_or_sell_common_attrs_from_csv_tx(
    csv_tx: &CsvTx,
    buy_or_sell: &str,
) -> Result<CommonBuySellAttrs, String> {
    let shares = csv_tx.shares.ok_or("\"shares\" not found")?;
    let amount_per_share = csv_tx.get_sane_amount_per_share()?;
    let commission = csv_tx.commission.unwrap_or(Decimal::ZERO);

    let curr_and_rate = get_valid_exchange_rate(
        CsvCol::TX_CURR,
        &csv_tx.tx_currency,
        CsvCol::TX_FX,
        &csv_tx.tx_curr_to_local_exchange_rate,
    )?
    .unwrap_or_else(|| CurrencyAndExchangeRate::default());

    let comm_curr_and_rate = get_valid_exchange_rate(
        CsvCol::COMMISSION_CURR,
        &csv_tx.commission_currency,
        CsvCol::COMMISSION_FX,
        &csv_tx.commission_curr_to_local_exchange_rate,
    )?;

    let specifics = CommonBuySellAttrs {
        shares: PosDecimal::try_from(shares)
            .map_err(|_| format!("{buy_or_sell} shares must be a positive value"))?,
        amount_per_share: GreaterEqualZeroDecimal::try_from(amount_per_share)
            .map_err(|_| {
                format!("{buy_or_sell} amount/share must not be negative")
            })?,
        commission: GreaterEqualZeroDecimal::try_from(commission)
            .map_err(|_| format!("{buy_or_sell} comission must not be negative"))?,
        tx_currency_and_rate: curr_and_rate,
        separate_commission_currency: comm_curr_and_rate,
    };
    Ok(specifics)
}

fn total_or_amount_per_share_to_gez(
    amount: TotalOrAmountPerShare<Decimal>,
    action_name: &str,
) -> Result<TotalOrAmountPerShare<GreaterEqualZeroDecimal>, String> {
    match amount {
        TotalOrAmountPerShare::Total(total) => {
            let gez = GreaterEqualZeroDecimal::try_from(total)
                .map_err(|_| {
                    format!(
                        "{action_name} \"{}\" must not be negative. Found {}",
                        CsvCol::TOTAL_AMOUNT, total
                    )
                })?;
            Ok(TotalOrAmountPerShare::Total(gez))
        }
        TotalOrAmountPerShare::AmountPerShare(amount_per_share) => {
            let gez = GreaterEqualZeroDecimal::try_from(amount_per_share)
                .map_err(|_| {
                    format!(
                        "{action_name} \"{}\" must not be negative. Found {}",
                        CsvCol::AMOUNT_PER_SHARE, amount_per_share
                    )
                })?;
            Ok(TotalOrAmountPerShare::AmountPerShare(gez))
        }
    }
}

impl TryFrom<CsvTx> for Tx {
    type Error = String;

    fn try_from(csv_tx: CsvTx) -> Result<Self, Self::Error> {
        if csv_tx.action.is_none() {
            return Err(format!(
                "\"action\" not specified. security: {:?}, trade date: {:?}",
                csv_tx.security, csv_tx.trade_date
            ));
        }

        let act_specs = match csv_tx.action.unwrap() {
            TxAction::Buy => {
                let specs = BuyTxSpecifics::from_common_buy_sell_attrs(
                    buy_or_sell_common_attrs_from_csv_tx(&csv_tx, "Buy")?,
                );
                TxActionSpecifics::Buy(specs)
            }
            TxAction::Sell => {
                let specs = SellTxSpecifics::from_common_buy_sell_attrs(
                    &(buy_or_sell_common_attrs_from_csv_tx(&csv_tx, "Sell")?),
                    csv_tx.specified_superficial_loss,
                );
                TxActionSpecifics::Sell(specs)
            }
            TxAction::Roc => {
                let amount_dec = csv_tx.get_sane_total_or_amount_per_share()
                    .map_err(|e| format!("RoC {e}"))?;
                let amount = total_or_amount_per_share_to_gez(amount_dec, "RoC")?;

                if let Some(shares) = csv_tx.shares {
                    // This will be confusing if we allow this to be specified.
                    return Err(format!("RoC should not specify shares (found {}). This amount is automatic \
                        (the current share balance)", shares));
                }

                let curr_and_rate = get_valid_exchange_rate(
                    CsvCol::TX_CURR,
                    &csv_tx.tx_currency,
                    CsvCol::TX_FX,
                    &csv_tx.tx_curr_to_local_exchange_rate,
                )?
                .unwrap_or_else(|| CurrencyAndExchangeRate::default());

                TxActionSpecifics::Roc(RocTxSpecifics {
                    amount: amount,
                    tx_currency_and_rate: curr_and_rate,
                })
            }
            TxAction::Sfla => {
                let amount: TotalAmount<Decimal> = csv_tx.get_sane_total_amount()
                    .map_err(|e| format!("SfLA {e}"))?;

                // Verify that this is either CAD or not specified
                let maybe_curr_and_rate = get_valid_exchange_rate(
                    CsvCol::TX_CURR,
                    &csv_tx.tx_currency,
                    CsvCol::TX_FX,
                    &csv_tx.tx_curr_to_local_exchange_rate,
                )?;
                if let Some(curr_and_rate) = maybe_curr_and_rate {
                    if !curr_and_rate.is_default() {
                        return Err("SfLA currency must be CAD/default".to_string());
                    }
                }

                let sfla_specs = match amount {
                    TotalAmount::ExplicitTotal(total) =>
                        SflaTxSpecifics::from_total(
                            PosDecimal::try_from(total)
                                .map_err(|_| {
                                    format!(
                                        "SfLA {} must be positive (found {})",
                                        CsvCol::TOTAL_AMOUNT,
                                        total
                                    )
                                })?,
                        ),
                    TotalAmount::TotalByShare(shares_and_amount) =>
                        SflaTxSpecifics::from_per_share(
                            PosDecimal::try_from(shares_and_amount.shares).map_err(|_| {
                                format!(
                                    "SfLA {} must be positive (found {})",
                                    CsvCol::SHARES,
                                    shares_and_amount.shares
                                )
                            })?,
                            PosDecimal::try_from(shares_and_amount.amount_per_share)
                                .map_err(|_| {
                                    format!(
                                        "SfLA {} must be positive (found {})",
                                        CsvCol::AMOUNT_PER_SHARE,
                                        shares_and_amount.amount_per_share
                                    )
                                })?,
                        ),
                };

                TxActionSpecifics::Sfla(sfla_specs)
            }
            TxAction::Split => TxActionSpecifics::Split(SplitTxSpecifics {
                ratio: csv_tx
                    .stock_split_ratio
                    .ok_or(format!("Split \"{}\" not found", CsvCol::SPLIT_RATIO))?,
            }),
        };

        let not_found_err =
            |col_name| -> String { format!("\"{col_name}\" not found") };

        let is_split = act_specs.action() == TxAction::Split;

        let tx = Tx {
            security: csv_tx
                .security
                .ok_or_else(|| not_found_err(CsvCol::SECURITY))?,
            trade_date: csv_tx
                .trade_date
                .ok_or_else(|| not_found_err(CsvCol::TRADE_DATE))?,
            settlement_date: csv_tx
                .settlement_date
                .ok_or_else(|| not_found_err(CsvCol::SETTLEMENT_DATE))?,
            action_specifics: act_specs,
            memo: csv_tx.memo.unwrap_or_else(|| String::new()),
            affiliate: csv_tx.affiliate.unwrap_or_else(||
                // Unless otherwise specified, splits apply to all affiliates
                if is_split { Affiliate::global() } else { Affiliate::default() }
            ),
            read_index: csv_tx.read_index,
        };
        if tx.security.is_empty() {
            return Err(format!("\"{}\" was empty", CsvCol::SECURITY));
        }
        Ok(tx)
    }
}

fn populate_csvtx_fields_from_action_specifics(
    specs: &TxActionSpecifics,
    tx: &mut CsvTx,
) {
    let populate_buy_sell_common_attrs = |tx: &mut CsvTx, c: CommonBuySellAttrs| {
        tx.shares = Some(*c.shares);
        tx.amount_per_share = Some(*c.amount_per_share);
        tx.commission = Some(*c.commission);
        tx.tx_currency = Some(c.tx_currency_and_rate.currency.clone());
        tx.tx_curr_to_local_exchange_rate = if c.tx_currency_and_rate.is_default() {
            None
        } else {
            Some(*c.tx_currency_and_rate.exchange_rate)
        };
        tx.commission_currency = match &c.separate_commission_currency {
            Some(c_a_r) => Some(c_a_r.currency.clone()),
            None => None,
        };
        tx.commission_curr_to_local_exchange_rate =
            match &c.separate_commission_currency {
                Some(c_a_r) => {
                    if c_a_r.is_default() {
                        None
                    } else {
                        Some(*c_a_r.exchange_rate)
                    }
                }
                None => None,
            };
    };

    match specs {
        TxActionSpecifics::Buy(s) => {
            tx.action = Some(TxAction::Buy);
            populate_buy_sell_common_attrs(tx, s.common_buy_sell_attrs());
        }
        TxActionSpecifics::Sell(s) => {
            tx.action = Some(TxAction::Sell);
            populate_buy_sell_common_attrs(tx, s.common_buy_sell_attrs());
            tx.specified_superficial_loss = s.specified_superficial_loss.clone();
        }
        TxActionSpecifics::Roc(s) => {
            tx.action = Some(TxAction::Roc);
            tx.amount_per_share = s.amount_per_held_share().map(|v| *v);
            tx.total_amount = s.total_amount().map(|v| *v);
            tx.tx_currency = Some(s.tx_currency_and_rate.currency.clone());
            tx.tx_curr_to_local_exchange_rate =
                if s.tx_currency_and_rate.is_default() {
                    None
                } else {
                    Some(*s.tx_currency_and_rate.exchange_rate)
                };
        }
        TxActionSpecifics::Sfla(s) => {
            tx.action = Some(TxAction::Sfla);
            tx.shares = s.shares_affected().map(|s| *s);
            tx.amount_per_share = s.amount_per_share().map(|s| *s);
            tx.total_amount = s.explicit_total_amount().map(|s| *s);
        }
        TxActionSpecifics::Split(s) => {
            tx.action = Some(TxAction::Split);
            tx.stock_split_ratio = Some(s.ratio.clone());
        }
    }
}

impl PartialOrd for Tx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let date_cmp = self.settlement_date.cmp(&other.settlement_date);
        match date_cmp {
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater => Some(date_cmp),
            std::cmp::Ordering::Equal => {
                Some(self.read_index.cmp(&other.read_index))
            }
        }
    }
}

impl Ord for Tx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

// MARK: testlib

#[cfg(test)]
pub mod testlib {
    use lazy_static::lazy_static;
    use time::{Date, Duration};

    use crate::{
        gezdec,
        portfolio::{Affiliate, Currency},
        util::{
            date::{parse_standard_date, pub_testlib::doy_date},
            decimal::GreaterEqualZeroDecimal,
        },
    };

    use super::{CsvTx, SFLInput, SplitRatio, Tx, TxAction};

    // This isn't terribly rust-esque, but it makes tests a bit more brief
    // than using Option everywhere within the tests.
    pub const MAGIC_DEFAULT_I32: i32 = -1234;
    pub const MAGIC_DEFAULT_U32: u32 = 91284091;
    pub const MAGIC_DEFAULT_AF_NAME: &str = "unset AF";

    lazy_static! {
        pub static ref MAGIC_DEFAULT_GEZ: GreaterEqualZeroDecimal =
            gezdec!(123456789.987654321);
        pub static ref MAGIC_DEFAULT_DATE: Date =
            parse_standard_date("1970-01-01").unwrap();
        pub static ref MAGIC_DEFAULT_CURRENCY: Currency =
            Currency::new("magic_default");
    }

    pub fn default_sec() -> String {
        "FOO".to_string()
    }

    pub fn mk_date(i: i32) -> Date {
        doy_date(2017, i as i64)
    }

    // Test Tx
    pub struct TTx {
        pub sec: String,

        pub t_day: i32, // An arbitrary offset day. Convenience for tdate
        pub t_date: Date, // Defaults to 2 days before sdate
        pub s_yr: u32,  // Year. Convenience for s_date. Must be combined with s_doy
        pub s_doy: i32, // Day of Year. convenience for s_date. Must be combined with s_yr
        pub s_date: Date, // Defaults to 2 days after t_date/t_day

        pub act: TxAction, // Required

        pub shares: GreaterEqualZeroDecimal,
        pub price: GreaterEqualZeroDecimal,
        pub t_amt: GreaterEqualZeroDecimal, // Total Amount
        pub comm: GreaterEqualZeroDecimal,
        pub curr: Currency,
        pub fx_rate: GreaterEqualZeroDecimal,
        pub comm_curr: Currency,
        pub comm_fx_rate: GreaterEqualZeroDecimal,
        pub memo: String,
        pub af: Affiliate,
        pub af_name: &'static str,
        pub sfl: Option<SFLInput>,
        pub split: Option<SplitRatio>,

        pub read_index: u32,
    }

    impl TTx {
        pub fn x(&self) -> Tx {
            let fx_rate = if self.fx_rate == *MAGIC_DEFAULT_GEZ {
                gezdec!(1)
            } else {
                self.fx_rate
            };
            let affiliate = if self.af_name == MAGIC_DEFAULT_AF_NAME {
                self.af.clone()
            } else {
                Affiliate::from_strep(self.af_name)
            };

            // Dates
            let mut trade_date = if self.t_day != MAGIC_DEFAULT_I32 {
                assert_eq!(self.t_date, *MAGIC_DEFAULT_DATE);
                mk_date(self.t_day)
            } else {
                self.t_date
            };

            let mut settlement_date = if self.s_yr != MAGIC_DEFAULT_U32 {
                assert_eq!(self.s_date, *MAGIC_DEFAULT_DATE);
                doy_date(self.s_yr, self.s_doy.into())
            } else {
                assert_eq!(self.s_doy, MAGIC_DEFAULT_I32);
                self.s_date
            };

            if settlement_date == *MAGIC_DEFAULT_DATE
                && trade_date != *MAGIC_DEFAULT_DATE
            {
                settlement_date = trade_date.saturating_add(Duration::days(2))
            } else if trade_date == *MAGIC_DEFAULT_DATE
                && settlement_date != *MAGIC_DEFAULT_DATE
            {
                trade_date = settlement_date.saturating_sub(Duration::days(2))
            } else if trade_date == *MAGIC_DEFAULT_DATE
                && settlement_date == *MAGIC_DEFAULT_DATE
            {
                panic!("TTx.x: Both trade and settlement dates are unset");
            }

            let curr = if self.curr == *MAGIC_DEFAULT_CURRENCY {
                Currency::cad()
            } else {
                self.curr.clone()
            };

            let comm_curr = if self.comm_curr == *MAGIC_DEFAULT_CURRENCY {
                None
            } else {
                Some(self.comm_curr.clone())
            };
            let comm_fx_rate = if self.comm_fx_rate == *MAGIC_DEFAULT_GEZ {
                None
            } else {
                Some(*self.comm_fx_rate)
            };

            let csv_tx = CsvTx {
                security: Some(if self.sec.is_empty() {
                    default_sec()
                } else {
                    self.sec.clone()
                }),
                trade_date: Some(trade_date),
                settlement_date: Some(settlement_date),
                action: Some(self.act),
                shares: if self.shares != *MAGIC_DEFAULT_GEZ {
                    Some(*self.shares)
                } else {
                    None
                },
                amount_per_share: if self.price != *MAGIC_DEFAULT_GEZ {
                    Some(*self.price)
                } else {
                    None
                },
                total_amount: if self.t_amt != *MAGIC_DEFAULT_GEZ {
                    Some(*self.t_amt)
                } else {
                    None
                },
                commission: if self.comm != *MAGIC_DEFAULT_GEZ {
                    Some(*self.comm)
                } else {
                    None
                },
                tx_currency: Some(curr),
                tx_curr_to_local_exchange_rate: Some(*fx_rate),
                commission_currency: comm_curr,
                commission_curr_to_local_exchange_rate: comm_fx_rate,
                memo: if self.memo.is_empty() {
                    None
                } else {
                    Some(self.memo.clone())
                },
                affiliate: Some(affiliate),
                specified_superficial_loss: self.sfl.clone(),
                stock_split_ratio: self.split.clone(),
                read_index: self.read_index,
            };
            Tx::try_from(csv_tx).unwrap()
        }

        pub fn d() -> Self {
            Self::default()
        }
    }

    impl Default for TTx {
        fn default() -> Self {
            TTx {
                sec: String::new(),

                t_day: MAGIC_DEFAULT_I32,
                t_date: *MAGIC_DEFAULT_DATE,
                s_yr: MAGIC_DEFAULT_U32, // Year. Convenience for s_date. Must be combined with s_doy
                s_doy: MAGIC_DEFAULT_I32, // Day of Year. convenience for s_date. Must be combined with s_yr
                s_date: *MAGIC_DEFAULT_DATE, // Defaults to 2 days after t_date/t_day

                act: TxAction::Roc, // Required. We have to pick one though

                shares: *MAGIC_DEFAULT_GEZ,
                price: *MAGIC_DEFAULT_GEZ,
                t_amt: *MAGIC_DEFAULT_GEZ,
                comm: *MAGIC_DEFAULT_GEZ,
                curr: MAGIC_DEFAULT_CURRENCY.clone(),
                fx_rate: *MAGIC_DEFAULT_GEZ,
                comm_curr: MAGIC_DEFAULT_CURRENCY.clone(),
                comm_fx_rate: *MAGIC_DEFAULT_GEZ,
                memo: String::new(),
                af: Affiliate::default(),
                af_name: MAGIC_DEFAULT_AF_NAME,
                sfl: None,
                split: None,

                read_index: 0,
            }
        }
    }
}

// MARK: Tests
#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::Date;

    use crate::pdec;
    use crate::portfolio::{
        Affiliate, Currency, CurrencyAndExchangeRate, SellTxSpecifics,
        SflaTxSpecifics,
    };
    use crate::testlib::assert_big_struct_eq;
    use crate::util::date;
    use crate::util::decimal::{GreaterEqualZeroDecimal, PosDecimal};
    use crate::{testlib::assert_vec_eq, util::date::parse_standard_date};

    use super::{
        BuyTxSpecifics, CsvTx, RocTxSpecifics, SFLInput, SplitRatio, TotalOrAmountPerShare, Tx, TxAction
    };

    pub const DEFAULT_SECURITY: &str = "FOO";

    pub fn tx_default() -> CsvTx {
        CsvTx {
            security: Some(DEFAULT_SECURITY.to_string()),
            trade_date: Some(Date::MIN),
            settlement_date: Some(Date::MIN),
            action: Some(TxAction::Buy),
            shares: Some(dec!(1)),
            amount_per_share: Some(dec!(0)),
            ..CsvTx::default()
        }
    }

    fn pos(d: Decimal) -> PosDecimal {
        PosDecimal::try_from(d).unwrap()
    }

    fn gez(d: Decimal) -> GreaterEqualZeroDecimal {
        GreaterEqualZeroDecimal::try_from(d).unwrap()
    }

    #[test]
    fn test_split_ratio() {
        // typical ratio
        let r = SplitRatio::parse("2-for-1").unwrap();
        assert_eq!(
            r,
            SplitRatio {
                pre_split: pdec!(1),
                post_split: pdec!(2),
                reverse_integer_only: false,
            }
        );
        assert_eq!(r.to_string(), "2-for-1");
        assert!(!r.is_reverse_split());

        // Padding/whitespace removal
        assert_eq!(r, SplitRatio::parse(" 2-for-1 ").unwrap());

        // Reverse split without decimals
        let r = SplitRatio::parse("1-for-2").unwrap();
        assert_eq!(
            r,
            SplitRatio {
                pre_split: pdec!(2),
                post_split: pdec!(1),
                reverse_integer_only: true,
            }
        );
        assert!(r.is_reverse_split());

        // Reverse split with decimals
        let r = SplitRatio::parse("1.0-for-2").unwrap();
        assert_eq!(
            r,
            SplitRatio {
                pre_split: pdec!(2),
                post_split: pdec!(1),
                reverse_integer_only: false,
            }
        );
        assert!(r.is_reverse_split());

        let r = SplitRatio::parse("1-for-2.0").unwrap();
        assert_eq!(
            r,
            SplitRatio {
                pre_split: pdec!(2),
                post_split: pdec!(1),
                reverse_integer_only: false,
            }
        );
        assert!(r.is_reverse_split());

        let r = SplitRatio::parse("1.5-for-2.5").unwrap();
        assert_eq!(
            r,
            SplitRatio {
                pre_split: pdec!(2.5),
                post_split: pdec!(1.5),
                reverse_integer_only: false,
            }
        );
        assert!(r.is_reverse_split());

        // 1:1 (edge case. No one will do this)
        assert!(!SplitRatio::parse("1-for-1").unwrap().is_reverse_split());

        // Format precision
        assert_eq!(
            format!("{:.2}", SplitRatio::parse("2-for-1").unwrap()),
            "2.00-for-1.00"
        );
        assert_eq!(
            format!("{}", SplitRatio::parse("2.0-for-1.0").unwrap()),
            "2-for-1"
        );
        // Reverse split with decimal support must preserve this in re-render
        // (such that re-parsing it gives the same result).
        assert_eq!(
            format!("{}", SplitRatio::parse("1.00-for-2.00").unwrap()),
            "1.0-for-2.0"
        );
        // Non-integers will render in their default precision (which will be
        // as they were parsed).
        assert_eq!(
            format!("{}", SplitRatio::parse("2.1000-for-1.1000").unwrap()),
            "2.1000-for-1.1000"
        );

        // Errors
        SplitRatio::parse("2:1").unwrap_err();
        SplitRatio::parse("-1-for-2").unwrap_err();
    }

    #[test]
    fn test_get_sane_opt_amount_per_share() {
        let e = |s: &str| Err(s.to_string());
        let d = |dec: Decimal| Ok(Some(dec));
        let di = |i: i32| Ok(Some(Decimal::from(i)));

        let params: Vec<(Option<i32>, Option<i32>, Option<i32>,
                         Result<Option<Decimal>, String>)> = vec![
            // Shares,  per share, total,      result
            (  None,    None,      None,       Ok(None)  ),
            (  None,    None,      Some(10),   e("Unable to compute amount per share. 'total amount' specified, but 'shares' not found")  ),
            (  None,    Some(2),   None,       di(2) ),
            (  None,    Some(2),   Some(10),   e("Both 'amount/share' and 'total amount' were specified. Only one may be specified")  ),

            (  Some(2), None,      None,       Ok(None)  ),
            (  Some(2), None,      Some(10),   di(5)  ),
            (  Some(3), None,      Some(10),   d(dec!(3.3333333333333333333333333333))  ),
            (  Some(2), None,      Some(0),    di(0)  ),
            (  Some(0), None,      Some(10),   e("Unable to compute amount per share. 'total amount' specified, but 'shares' is zero")  ),
            (  Some(2), Some(5),   None,       di(5)  ),
            (  Some(2), Some(5),   Some(10),   e("Both 'amount/share' and 'total amount' were specified. Only one may be specified")  ),
        ];

        let mut errs = vec![];
        for (shares, aps, total, exp_result) in params {
            let mut tx = CsvTx::default();
            tx.shares = shares.map(Decimal::from);
            tx.amount_per_share = aps.map(Decimal::from);
            tx.total_amount = total.map(Decimal::from);

            let result = tx.get_sane_opt_amount_per_share();
            if tx.get_sane_opt_amount_per_share() != exp_result {
                errs.push(
                    format!("shares: {:?}, aps: {:?}, total: {:?} -> {:?}. Was not {:?}",
                            shares, aps, total, result, exp_result));
            }
        }
        assert_eq!(errs.len(), 0, "Errors: {:#?}", errs);
    }

    #[test]
    fn test_tx_order() {
        let doy_date = |day| date::pub_testlib::doy_date(2024, day);
        let sdoy_date = |day| Some(doy_date(day));

        #[rustfmt::skip]
        let mut csv_txs = vec![
            CsvTx{settlement_date: sdoy_date(4), read_index: 2, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(5), read_index: 1, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(2), read_index: 4, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(4), read_index: 3, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(1), read_index: 5, ..tx_default()},
        ];
        let mut txs: Vec<Tx> =
            csv_txs.iter().map(|c_tx| Tx::try_from(c_tx.clone()).unwrap()).collect();

        #[rustfmt::skip]
        let exp_csv_txs = vec![
            CsvTx{settlement_date: sdoy_date(1), read_index: 5, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(2), read_index: 4, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(4), read_index: 2, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(4), read_index: 3, ..tx_default()},
            CsvTx{settlement_date: sdoy_date(5), read_index: 1, ..tx_default()},
        ];
        let exp_txs: Vec<Tx> = exp_csv_txs
            .iter()
            .map(|c_tx| Tx::try_from(c_tx.clone()).unwrap())
            .collect();

        csv_txs.sort();
        assert_vec_eq(csv_txs, exp_csv_txs);

        // Tx should have the same order
        txs.sort();
        assert_vec_eq(txs, exp_txs);
    }

    fn fully_populated_csvtx(action: TxAction, use_total: bool) -> CsvTx {
        CsvTx {
            security: Some("FOO".to_string()),
            trade_date: Some(parse_standard_date("2022-10-20").unwrap()),
            settlement_date: Some(parse_standard_date("2022-10-21").unwrap()),
            action: Some(action),
            shares: Some(dec!(123.1)),
            amount_per_share: if use_total { None } else { Some(dec!(10.1)) },
            total_amount: if use_total { Some(dec!(10.1)*dec!(123.1)) } else { None },
            commission: Some(dec!(1.01)),
            tx_currency: Some(Currency::usd()),
            tx_curr_to_local_exchange_rate: Some(dec!(1.21)),
            commission_currency: Some(Currency::new("EUR")),
            commission_curr_to_local_exchange_rate: Some(dec!(2.01)),
            memo: Some("A memo".to_string()),
            affiliate: Some(Affiliate::default_registered()),
            // Ignored for buy
            specified_superficial_loss: Some(SFLInput::req_from_dec(
                dec!(-2.5),
                false,
            )),
            // Ignored for anything but split
            stock_split_ratio: Some(SplitRatio::parse("2-for-1").unwrap()),
            read_index: 5,
        }
    }

    fn barebones_valid_sample_csvtx(action: TxAction) -> CsvTx {
        CsvTx {
            security: Some("FOO".to_string()),
            trade_date: Some(parse_standard_date("2022-10-20").unwrap()),
            settlement_date: Some(parse_standard_date("2022-10-21").unwrap()),
            action: Some(action),
            shares: Some(dec!(123.1)),
            amount_per_share: Some(dec!(10.1)),
            read_index: 5,
            ..CsvTx::default()
        }
    }

    fn do_test_csvtx_to_tx_full(use_total_amount: bool) {
        // Fully explicit buy
        let buy_csvtx = fully_populated_csvtx(TxAction::Buy, use_total_amount);
        let sell_csvtx = fully_populated_csvtx(TxAction::Sell, use_total_amount);
        let mut sfla_csvtx = fully_populated_csvtx(TxAction::Sfla, use_total_amount);
        sfla_csvtx.tx_curr_to_local_exchange_rate = None;
        sfla_csvtx.tx_currency = None;
        if use_total_amount {
            sfla_csvtx.shares = None;
        }
        let sfla_csvtx = sfla_csvtx; // finalize

        let mut roc_csvtx = fully_populated_csvtx(TxAction::Roc, use_total_amount);
        roc_csvtx.shares = None;
        let roc_csvtx = roc_csvtx;

        let exp_buy_tx = Tx {
            security: buy_csvtx.security.clone().unwrap(),
            trade_date: buy_csvtx.trade_date.clone().unwrap(),
            settlement_date: buy_csvtx.settlement_date.clone().unwrap(),
            action_specifics: super::TxActionSpecifics::Buy(BuyTxSpecifics {
                shares: pos(buy_csvtx.shares.unwrap()),
                amount_per_share: gez(dec!(10.1)),
                commission: gez(buy_csvtx.commission.unwrap()),
                tx_currency_and_rate: CurrencyAndExchangeRate::rq_new(
                    buy_csvtx.tx_currency.clone().unwrap(),
                    pos(buy_csvtx.tx_curr_to_local_exchange_rate.unwrap()),
                ),
                separate_commission_currency: Some(CurrencyAndExchangeRate::rq_new(
                    buy_csvtx.commission_currency.clone().unwrap(),
                    pos(buy_csvtx.commission_curr_to_local_exchange_rate.unwrap()),
                )),
            }),
            memo: buy_csvtx.memo.clone().unwrap(),
            affiliate: buy_csvtx.affiliate.clone().unwrap(),
            read_index: buy_csvtx.read_index,
        };

        let tx = Tx::try_from(buy_csvtx.clone()).unwrap();
        assert_big_struct_eq(&tx, &exp_buy_tx);
        // Osclilating from Tx -> CsvTx -> Tx should be stable (not necessarily in the inverse though)
        assert_big_struct_eq(Tx::try_from(tx.to_csvtx()).unwrap(), tx);

        let mut exp_sell_tx = exp_buy_tx.clone();
        exp_sell_tx.action_specifics =
            super::TxActionSpecifics::Sell(SellTxSpecifics {
                shares: pos(sell_csvtx.shares.unwrap()),
                amount_per_share: gez(dec!(10.1)),
                commission: gez(sell_csvtx.commission.unwrap()),
                tx_currency_and_rate: CurrencyAndExchangeRate::rq_new(
                    sell_csvtx.tx_currency.clone().unwrap(),
                    pos(sell_csvtx.tx_curr_to_local_exchange_rate.unwrap()),
                ),
                separate_commission_currency: Some(CurrencyAndExchangeRate::rq_new(
                    sell_csvtx.commission_currency.clone().unwrap(),
                    pos(sell_csvtx.commission_curr_to_local_exchange_rate.unwrap()),
                )),
                specified_superficial_loss: sell_csvtx
                    .specified_superficial_loss
                    .clone(),
            });

        let tx = Tx::try_from(sell_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_sell_tx);
        assert_big_struct_eq(Tx::try_from(tx.to_csvtx()).unwrap(), tx);

        let mut exp_sfla_tx = exp_buy_tx.clone();
        exp_sfla_tx.action_specifics = if use_total_amount {
            super::TxActionSpecifics::Sfla(SflaTxSpecifics::from_total(
                pos(sfla_csvtx.total_amount.unwrap()),
            ))
        } else {
            super::TxActionSpecifics::Sfla(SflaTxSpecifics::from_per_share(
                pos(sfla_csvtx.shares.unwrap()),
                pos(sfla_csvtx.amount_per_share.unwrap()),
            ))
        };

        let tx = Tx::try_from(sfla_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_sfla_tx);
        assert_big_struct_eq(Tx::try_from(tx.to_csvtx()).unwrap(), tx);

        let mut exp_roc_tx = exp_buy_tx.clone();
        exp_roc_tx.action_specifics =
            super::TxActionSpecifics::Roc(RocTxSpecifics {
                amount: if roc_csvtx.amount_per_share.is_some() {
                    TotalOrAmountPerShare::AmountPerShare(
                        gez(roc_csvtx.amount_per_share.unwrap()))
                } else {
                    TotalOrAmountPerShare::Total(
                        gez(roc_csvtx.total_amount.unwrap()))
                },
                tx_currency_and_rate: CurrencyAndExchangeRate::rq_new(
                    roc_csvtx.tx_currency.clone().unwrap(),
                    pos(roc_csvtx.tx_curr_to_local_exchange_rate.unwrap()),
                ),
            });

        let tx = Tx::try_from(roc_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_roc_tx);
        assert_big_struct_eq(Tx::try_from(tx.to_csvtx()).unwrap(), tx);
    }

    #[test]
    fn test_csvtx_to_tx_full_amt_per_share() {
        do_test_csvtx_to_tx_full(false);
    }

    #[test]
    fn test_csvtx_to_tx_full_total_amt() {
        do_test_csvtx_to_tx_full(true);
    }

    #[test]
    fn test_csvtx_to_tx_defaults_and_optionals() {
        let buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);

        let exp_buy_tx = Tx {
            security: buy_csvtx.security.clone().unwrap(),
            trade_date: buy_csvtx.trade_date.clone().unwrap(),
            settlement_date: buy_csvtx.settlement_date.clone().unwrap(),
            action_specifics: super::TxActionSpecifics::Buy(BuyTxSpecifics {
                shares: pos(buy_csvtx.shares.unwrap()),
                amount_per_share: gez(buy_csvtx.amount_per_share.unwrap()),
                commission: gez(dec!(0)),
                tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                separate_commission_currency: None,
            }),
            memo: String::new(),
            affiliate: Affiliate::default(),
            read_index: buy_csvtx.read_index,
        };

        let tx = Tx::try_from(buy_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_buy_tx);
        assert_big_struct_eq(Tx::try_from(tx.to_csvtx()).unwrap(), tx);

        let sell_csvtx = barebones_valid_sample_csvtx(TxAction::Sell);

        let exp_sell_tx = Tx {
            security: sell_csvtx.security.clone().unwrap(),
            trade_date: sell_csvtx.trade_date.clone().unwrap(),
            settlement_date: sell_csvtx.settlement_date.clone().unwrap(),
            action_specifics: super::TxActionSpecifics::Sell(SellTxSpecifics {
                shares: pos(sell_csvtx.shares.unwrap()),
                amount_per_share: gez(sell_csvtx.amount_per_share.unwrap()),
                commission: gez(dec!(0)),
                tx_currency_and_rate: CurrencyAndExchangeRate::default(),
                separate_commission_currency: None,
                specified_superficial_loss: None,
            }),
            memo: String::new(),
            affiliate: Affiliate::default(),
            read_index: sell_csvtx.read_index,
        };

        let tx = Tx::try_from(sell_csvtx).unwrap();
        assert_big_struct_eq(&tx, &exp_sell_tx);
        assert_big_struct_eq(Tx::try_from(tx.to_csvtx()).unwrap(), tx);
    }

    #[test]
    fn test_csvtx_to_tx_err() {
        // This could be one of any of the errors.
        let _ = Tx::try_from(CsvTx::default()).unwrap_err();

        // Most errors are going to be pretty straightforward, because
        // of hard type constraints (like PosDecimal, etc) and
        // non-optional types in Tx. There are a lot of such cases,
        // so we'll skip most of those.

        // Check one of the missing optional cases (security)
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.security = None;

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"security\" not found");

        // Also check missing trade date (in legacy, only 'date' aka settlement date
        // was provided. This is no longer valid).
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.trade_date = None;

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"trade date\" not found");

        // Both total amount and amount/share
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.total_amount = Some(dec!(5.0));

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "Both 'amount/share' and 'total amount' were specified. Only one may be specified");

        // Interesting Cases:
        // - no action (because it is the only attr we call unwrap() on.
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.action = None;

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"action\" not specified. security: Some(\"FOO\"), trade date: Some(2022-10-20)");

        // - currency but no exchange rate (and inverse)
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_currency = Some(Currency::usd());

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(
            err,
            "\"currency\" specified but \"exchange rate\" not found"
        );

        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_curr_to_local_exchange_rate = Some(dec!(1.2));

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(
            err,
            "\"exchange rate\" specified but \"currency\" not found"
        );

        // (double check that CAD doesn't need a rate, and if it does, it must be 1)
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_currency = Some(Currency::cad());

        let tx = Tx::try_from(buy_csvtx).unwrap();
        assert_eq!(
            tx.buy_specifics().unwrap().tx_currency_and_rate.currency,
            Currency::cad()
        );
        assert_eq!(
            tx.buy_specifics().unwrap().tx_currency_and_rate.exchange_rate,
            pdec!(1)
        );

        // Invalid default rate
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.tx_currency = Some(Currency::cad());
        buy_csvtx.tx_curr_to_local_exchange_rate = Some(dec!(1.2));

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(
            err,
            "Default currency (CAD) exchange rate was not 1 (was 1.2)"
        );

        // - comm currency but no rate (and inverse). Just sanity check we're using
        // the same function.
        let mut buy_csvtx = barebones_valid_sample_csvtx(TxAction::Buy);
        buy_csvtx.commission_currency = Some(Currency::usd());

        let err = Tx::try_from(buy_csvtx).unwrap_err();
        assert_eq!(err, "\"commission currency\" specified but \"commission exchange rate\" not found");

        // Non-CAD SFLA
        let mut sfla_csvtx = barebones_valid_sample_csvtx(TxAction::Sfla);
        sfla_csvtx.tx_currency = Some(Currency::usd());
        sfla_csvtx.tx_curr_to_local_exchange_rate = Some(dec!(1.5));
        let err = Tx::try_from(sfla_csvtx).unwrap_err();
        assert_eq!(err, "SfLA currency must be CAD/default");

        // Invalid RoC, with specified shares
        let roc_csvtx = barebones_valid_sample_csvtx(TxAction::Roc);
        let err = Tx::try_from(roc_csvtx).unwrap_err();
        assert_eq!(err, "RoC should not specify shares (found 123.1). This amount is automatic (the current share balance)");
    }
}
