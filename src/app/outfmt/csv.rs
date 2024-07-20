use std::{fs::File, io, path::PathBuf};

use crate::util::{os::mk_writable_dir, rw::WriteHandle};

use super::model::{AcbWriter, OutputType};

enum WriteMode {
    Directory(PathBuf),
    Writer(WriteHandle),
}

pub struct CsvWriter {
    mode: WriteMode,
}

impl CsvWriter {
    pub fn new_to_output_dir(out_dir: &String) -> Result<CsvWriter, io::Error> {
        let dir_path = PathBuf::from(out_dir);
        mk_writable_dir(&dir_path)?;
        Ok(CsvWriter {
            mode: WriteMode::Directory(dir_path),
        })
    }

    pub fn new_to_writer(wh: WriteHandle) -> CsvWriter {
        CsvWriter {
            mode: WriteMode::Writer(wh),
        }
    }

    fn get_writer(
        &mut self,
        out_type: OutputType,
        name: &str,
    ) -> Result<Box<dyn std::io::Write>, super::model::Error> {
        match &self.mode {
            WriteMode::Directory(out_dir) => {
                let file_name = match out_type {
                    OutputType::Transactions => format!("{name}.csv"),
                    OutputType::AggregateGains => "aggregate-gains.csv".to_string(),
                    OutputType::Costs => {
                        format!(
                            "{}-costs.csv",
                            name.to_lowercase().replace(" ", "-")
                        )
                    }
                    OutputType::Raw => format!("{name}.csv"),
                };

                let file_path = out_dir.join(PathBuf::from(file_name));
                let fp = File::create(file_path.clone()).map_err(|e| {
                    format!("Failed to create {:?}: {}", file_path.to_str(), e)
                })?;

                Ok(Box::new(fp))
            }
            WriteMode::Writer(write_handle) => Ok(Box::new(write_handle.clone())),
        }
    }
}

impl AcbWriter for CsvWriter {
    fn print_render_table(
        &mut self,
        out_type: OutputType,
        name: &str,
        table_model: &crate::portfolio::render::RenderTable,
    ) -> Result<(), super::model::Error> {
        let writer = self.get_writer(out_type, name)?;
        let mut csv_w =
            csv::WriterBuilder::new().has_headers(true).from_writer(writer);

        csv_w.write_record(&table_model.header).map_err(|e| e.to_string())?;
        for row in &table_model.rows {
            csv_w.write_record(row).map_err(|e| e.to_string())?;
        }
        if table_model.footer.len() > 0 {
            csv_w.write_record(&table_model.footer).map_err(|e| e.to_string())?;
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
