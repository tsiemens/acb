use super::sheet_common::SheetParseError;

mod broker_tx;
mod fx_tracker;

// Individual brokers
pub mod etrade;
pub mod questrade;

pub struct SheetToTxsErr {
    // Note that both of these can be populated at the same time.
    // txs is an incomplete set of the parsed txs.
    pub txs: Option<Vec<BrokerTx>>,
    pub errors: Vec<SheetParseError>,
}

pub use broker_tx::*;
pub use fx_tracker::*;