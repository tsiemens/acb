// use crate::{fx::io::RateLoader, log::WriteHandle, portfolio::CsvTx};

type Error = String;

// Takes CsvTxs parsed from a CSV, and loads any missing exchange
// rates directly into them.
// This will be dependent on if any currency is set to USD, but the rate
// is None.
pub fn load_tx_rates(
    // csv_txs: &mut Vec<CsvTx>,
    // rate_loader: &mut RateLoader,
    // err_stream: &mut WriteHandle,
    ) -> Result<(), Error> {
    todo!();
}