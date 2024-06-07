use std::io::Write;

use tabled::settings::{
    object::{Cell, Columns, Rows},
    style::On,
    Alignment, Border,
};

use crate::{portfolio::render::RenderTable, util::rw::WriteHandle};

use super::model::{AcbWriter, OutputType};

pub struct TextWriter {
    w: WriteHandle,
}

impl TextWriter {
    pub fn new(w: WriteHandle) -> TextWriter {
        TextWriter { w: w }
    }
}

struct CellBorder {
    top: char,
    bottom: char,
    left: char,
    right: char,
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
}

impl CellBorder {
    pub fn to_border(&self) -> Border<On, On, On, On> {
        Border::full(
            self.top,
            self.bottom,
            self.left,
            self.right,
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
        )
    }

    pub fn none() -> CellBorder {
        Self {
            top: ' ',
            bottom: ' ',
            left: ' ',
            right: ' ',
            top_left: ' ',
            top_right: ' ',
            bottom_left: ' ',
            bottom_right: ' ',
        }
    }
}

impl Default for CellBorder {
    fn default() -> Self {
        Self {
            top: '-',
            bottom: '-',
            left: '|',
            right: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        }
    }
}

impl AcbWriter for TextWriter {
    fn print_render_table(
        &mut self,
        out_type: OutputType,
        name: &str,
        table_model: &RenderTable,
    ) -> Result<(), super::model::Error> {
        let map_write_err = |e| format!("{e}");

        for err in &table_model.errors {
            writeln!(self.w, "[!] {}", err).map_err(map_write_err)?;
        }
        if table_model.errors.len() > 0 {
            writeln!(self.w, "Printing parsed information state:")
                .map_err(map_write_err)?;
        }

        let title = match out_type {
            OutputType::Transactions => format!("Transactions for {}", name),
            OutputType::AggregateGains => "Aggregate Gains".to_string(),
            OutputType::Costs => format!("{} Costs", name),
        };

        writeln!(self.w, "{}", title).map_err(map_write_err)?;

        let n_cols = table_model.header.len();
        let mut table_bldr = tabled::builder::Builder::default();
        table_bldr.push_record(
            table_model
                .header
                .iter()
                .map(|h| h.to_uppercase())
                .collect::<Vec<String>>(),
        );
        let n_rows = table_model.rows.len();
        for row in &table_model.rows {
            table_bldr.push_record(row);
        }

        let footer_sep_row: Option<usize> = if table_model.footer.len() > 0 {
            let mut split_line = Vec::with_capacity(table_model.footer.len());
            split_line.resize_with(table_model.footer.len(), || String::new());
            table_bldr.push_record(split_line);
            table_bldr.push_record(table_model.footer.clone());

            // footer row separator index
            Some(1 + n_rows)
        } else {
            None
        };

        let mut table = table_bldr.build();
        table.with(tabled::settings::Style::ascii());
        // Center the header
        table.modify(Rows::first(), Alignment::center());

        // Set top row borders (nothing on outer edge)
        table.modify(
            Rows::first(),
            CellBorder {
                top: ' ',
                top_left: ' ',
                top_right: ' ',
                ..Default::default()
            }
            .to_border(),
        );
        // Set left col borders
        table.modify(
            Columns::first(),
            CellBorder {
                left: ' ',
                top_left: '-',
                bottom_left: '-',
                ..Default::default()
            }
            .to_border(),
        );
        // Set right col borders
        table.modify(
            Columns::last(),
            CellBorder {
                right: ' ',
                top_right: '-',
                bottom_right: '-',
                ..Default::default()
            }
            .to_border(),
        );
        // Set upper-left corner borders
        table.modify(
            Cell::new(0, 0),
            CellBorder {
                left: ' ',
                top: ' ',
                top_right: ' ',
                top_left: ' ',
                bottom_left: '-',
                ..Default::default()
            }
            .to_border(),
        );
        // Set upper-right corner borders
        table.modify(
            Cell::new(0, n_cols - 1),
            CellBorder {
                right: ' ',
                top: ' ',
                top_right: ' ',
                top_left: ' ',
                bottom_right: '-',
                ..Default::default()
            }
            .to_border(),
        );

        // Set up the footer borders
        // The footer acts as a smaller table under the main table,
        // separated by a single blank row.
        if let Some(sep_row) = footer_sep_row {
            let footer_row = sep_row + 1;
            // By default, make the separator row and footer invisible,
            // without knocking out the bottom border of the main table.
            table.modify(
                Rows::single(sep_row),
                Border::new().set_left(' ').set_right(' '),
            );
            table.modify(Rows::single(footer_row), CellBorder::none().to_border());

            // Then, for each cell in the footer, give it a full border,
            // and re-add the borders into the separator, so it still looks
            // kind of connected to the main table.
            for (col, footer_cell) in table_model.footer.iter().enumerate() {
                if !footer_cell.is_empty() {
                    table.modify(
                        Cell::new(sep_row, col),
                        CellBorder::default().to_border(),
                    );
                    table.modify(
                        Cell::new(footer_row, col),
                        CellBorder::default().to_border(),
                    );
                }
            }
        }

        writeln!(self.w, "{table}").map_err(map_write_err)?;

        for note in &table_model.notes {
            writeln!(self.w, "{note}").map_err(map_write_err)?;
        }

        writeln!(self.w, "").map_err(map_write_err)?;
        Ok(())
    }
}
