use crate::{portfolio::render, util::rw::WriteHandle};

use super::outfmt::{model::{AcbWriter, OutputType}, text::TextWriter};

pub fn print_dummy_table() {
    let table_model = render::render_tx_table_model(
        Vec::new(), (), false,
    );

    let mut tw = TextWriter::new(WriteHandle::stdout_write_handle());
    let _ = tw.print_render_table(
        OutputType::Transactions, "Sample", table_model).unwrap();
}