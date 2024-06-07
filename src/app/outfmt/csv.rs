use std::{fs::File, io, path::PathBuf};

use crate::util::os::mk_writable_dir;

use super::model::{AcbWriter, OutputType};

pub struct CsvWriter {
    out_dir: PathBuf,
}

impl CsvWriter {
    pub fn new(out_dir: &String) -> Result<CsvWriter, io::Error> {
        let dir_path = PathBuf::from(out_dir);
        mk_writable_dir(&dir_path)?;
        Ok(CsvWriter { out_dir: dir_path })
    }
}

impl AcbWriter for CsvWriter {
    fn print_render_table(
        &mut self,
        out_type: OutputType,
        name: &str,
        table_model: &crate::portfolio::render::RenderTable,
    ) -> Result<(), super::model::Error> {
        let file_name = match out_type {
            OutputType::Transactions => format!("{name}.csv"),
            OutputType::AggregateGains => "aggregate-gains.csv".to_string(),
            OutputType::Costs => {
                format!("{}-costs.csv", name.to_lowercase().replace(" ", "-"))
            }
        };

        let file_path = self.out_dir.join(PathBuf::from(file_name));
        let fp = File::create(file_path.clone()).map_err(|e| {
            format!("Failed to create {:?}: {}", file_path.to_str(), e)
        })?;

        let mut csv_w = csv::WriterBuilder::new().has_headers(true).from_writer(fp);

        csv_w
            .write_record(&table_model.header)
            .map_err(|e| e.to_string())?;
        for row in &table_model.rows {
            csv_w.write_record(row).map_err(|e| e.to_string())?;
        }
        if table_model.footer.len() > 0 {
            csv_w
                .write_record(&table_model.footer)
                .map_err(|e| e.to_string())?;
        }

        let n_cols = table_model.header.len();

        for note in &table_model.notes {
            let mut note_record = Vec::<String>::with_capacity(n_cols);
            note_record.resize(n_cols, String::new());
            note_record[0] = note.clone();
            csv_w.write_record(note_record).map_err(|e| e.to_string())?;
        }

        csv_w.flush().map_err(|e| e.to_string())?;

        Ok(())
    }
}
