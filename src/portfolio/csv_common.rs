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
    pub const TOTAL_AMOUNT: &'static str = "total amount";
    pub const COMMISSION: &'static str = "commission";
    pub const TX_CURR: &'static str = "currency";
    pub const TX_FX: &'static str = "exchange rate";
    pub const COMMISSION_CURR: &'static str = "commission currency";
    pub const COMMISSION_FX: &'static str = "commission exchange rate";
    pub const SUPERFICIAL_LOSS: &'static str = "superficial loss";
    pub const SPLIT_RATIO: &'static str = "split ratio";
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
            CsvCol::TOTAL_AMOUNT,
            CsvCol::COMMISSION,
            CsvCol::TX_CURR,
            CsvCol::TX_FX,
            CsvCol::COMMISSION_CURR,
            CsvCol::COMMISSION_FX,
            CsvCol::SUPERFICIAL_LOSS,
            CsvCol::SPLIT_RATIO,
            CsvCol::AFFILIATE,
            CsvCol::MEMO,
        ]);
    }

    pub fn export_order_non_deprecated_cols() -> [&'static str; 16] {
        [
            CsvCol::SECURITY,
            CsvCol::TRADE_DATE,
            CsvCol::SETTLEMENT_DATE,
            CsvCol::ACTION,
            CsvCol::SHARES,
            CsvCol::AMOUNT_PER_SHARE,
            CsvCol::TOTAL_AMOUNT,
            CsvCol::COMMISSION,
            CsvCol::TX_CURR,
            CsvCol::TX_FX,
            CsvCol::COMMISSION_CURR,
            CsvCol::COMMISSION_FX,
            CsvCol::SUPERFICIAL_LOSS,
            CsvCol::SPLIT_RATIO,
            CsvCol::AFFILIATE,
            CsvCol::MEMO,
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::portfolio::csv_common::CsvCol;

    #[test]
    fn test_export_order_non_deprecated_cols() {
        let cols_to_write = std::collections::HashSet::from(
            CsvCol::export_order_non_deprecated_cols(),
        );
        let mut all_cols_minus_deprecated = CsvCol::get_csv_cols();
        all_cols_minus_deprecated.remove(CsvCol::LEGACY_SETTLEMENT_DATE);
        assert_eq!(cols_to_write, all_cols_minus_deprecated)
    }
}
