use std::io::Write;

use tabled::settings::{object::{Cell, Rows}, Border};

use crate::{portfolio::render::RenderTable, util::rw::WriteHandle};

use super::model::{AcbWriter, OutputType};

pub struct TextWriter {
    w: WriteHandle,
}

impl TextWriter {
    pub fn new(w: WriteHandle) -> TextWriter {
        TextWriter{w: w}
    }
}

impl AcbWriter for TextWriter {
    fn print_render_table(
        &mut self, out_type: OutputType, name: &str, table_model: RenderTable)
        -> Result<(), super::model::Error> {

        let map_write_err = |e| { format!("{e}") };

        for err in &table_model.errors {
            writeln!(self.w, "[!] {}", err)
                .map_err(map_write_err)?;
        }
        if table_model.errors.len() > 0 {
            writeln!(self.w, "Printing parsed information state:").map_err(map_write_err)?;
        }

        let title = match out_type {
            OutputType::Transactions => format!("Transactions for {}", name),
            OutputType::AggregateGains => "Aggregate Gains".to_string(),
            OutputType::Costs => format!("{} Costs", name),
        };

        writeln!(self.w, "{}", title).map_err(map_write_err)?;

        let n_cols = table_model.header.len();
        let mut table_bldr = tabled::builder::Builder::default();
        table_bldr.push_record(table_model.header);
        let n_rows = table_model.rows.len();
        for row in table_model.rows {
            table_bldr.push_record(row);
        }

        let footer_sep_row: Option<usize> = if table_model.footer.len() > 0 {
            let mut split_line = Vec::with_capacity(table_model.footer.len());
            split_line.resize_with(table_model.footer.len(), || String::new());
            table_bldr.push_record(split_line);
            table_bldr.push_record(table_model.footer);

            // footer row separator index
            Some(1 + n_rows)
        } else {
            None
        };

        let mut table = table_bldr.build();
        table.with(tabled::settings::Style::ascii());
        table.modify(Rows::first(), Border::full(
            ' ', '-', '|', '|', ' ', ' ', '+', '+'));
        table.modify(Cell::new(0, 0), Border::new().set_left(' '));
        table.modify(Cell::new(0, n_cols - 1), Border::new().set_right(' '));
        if let Some(sep_row) = footer_sep_row {
            table.modify(Rows::single(sep_row),
                         Border::new().set_left(' ').set_right(' '));
        }

        for note in table_model.notes {
            writeln!(self.w, "{note}").map_err(map_write_err)?;
        }
        writeln!(self.w, "{table}").map_err(map_write_err)?;

        Ok(())
    }
}