use super::TxDelta;

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

// TODO get actual type once defined.
type CumulativeCapitalGains = ();

pub fn render_tx_table_model(
	// deltas: Vec<TxDelta>, gains: CumulativeCapitalGains,
    // renderFullDollarValues: bool
	_: Vec<TxDelta>, _: CumulativeCapitalGains,
    _: bool
)
    -> RenderTable {
    RenderTable {
        header: vec!["Dummy".to_string(), "header".to_string()],
        rows: vec![vec!["R1C1".to_string(), "R1C2".to_string()]],
        footer: vec!["".to_string(), "======\nfooter1".to_string()],
        notes: vec!["A note".to_string()],
        errors: vec!["an error".to_string()],
    }
}