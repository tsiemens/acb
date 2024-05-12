use std::collections::HashSet;

pub struct CsvCol();
impl CsvCol {
    pub const SECURITY: &'static str = "security";
    pub const TRADE_DATE: &'static str = "trade date";
    pub const LEGACY_SETTLEMENT_DATE: &'static str = "date";
    pub const SETTLEMENT_DATE: &'static str = "settlement date";
    pub const ACTION: &'static str = "action";
    pub const SHARES: &'static str = "shares";
    pub const AMOUNT_PER_SHARE: &'static str = "amount/share";
    pub const COMMISSION: &'static str = "commission";
    pub const TX_CURR: &'static str = "currency";
    pub const TX_FX: &'static str = "exchange rate";
    pub const COMMISSION_CURR: &'static str = "commission currency";
    pub const COMMISSION_FX: &'static str = "commission exchange rate";
    pub const SUPERFICIAL_LOSS: &'static str = "superficial loss";
    pub const AFFILIATE: &'static str = "affiliate";
    pub const MEMO: &'static str = "memo";

    pub fn get_csv_cols() -> HashSet<&'static str> {
        return HashSet::from([
            CsvCol::SECURITY,
            CsvCol::TRADE_DATE,
            CsvCol::LEGACY_SETTLEMENT_DATE,
            CsvCol::SETTLEMENT_DATE,
            CsvCol::ACTION,
            CsvCol::SHARES,
            CsvCol::AMOUNT_PER_SHARE,
            CsvCol::COMMISSION,
            CsvCol::TX_CURR,
            CsvCol::TX_FX,
            CsvCol::COMMISSION_CURR,
            CsvCol::COMMISSION_FX,
            CsvCol::SUPERFICIAL_LOSS,
            CsvCol::AFFILIATE,
            CsvCol::MEMO,
        ]);
    }
}