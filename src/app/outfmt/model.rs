use crate::portfolio::render::RenderTable;

pub enum OutputType {
    Transactions,
    AggregateGains,
    Costs,
}

pub type Error = String;

pub trait AcbWriter {
    fn print_render_table(
        &mut self,
        out_type: OutputType,
        name: &str,
        table_model: &RenderTable,
    ) -> Result<(), Error>;
}
