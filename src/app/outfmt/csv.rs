use std::{fs::File, io, path::PathBuf};

use crate::util::{os::mk_writable_dir, rw::WriteHandle};

use super::model::{AcbWriter, OutputType};

fn print_render_table_to_csv<W: std::io::Write>(
    writer: W,
    table_model: &crate::portfolio::render::RenderTable,
) -> Result<(), super::model::Error> {
    let mut csv_w = csv::WriterBuilder::new().has_headers(true).from_writer(writer);

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

    pub fn get_csv_file_name(out_type: OutputType, name: &str) -> String {
        match out_type {
            OutputType::Transactions => format!("{name}.csv"),
            OutputType::AggregateGains => "aggregate-gains.csv".to_string(),
            OutputType::Costs => {
                format!("{}-costs.csv", name.to_lowercase().replace(" ", "-"))
            }
            OutputType::Raw => format!("{name}.csv"),
        }
    }

    fn get_writer(
        &mut self,
        out_type: OutputType,
        name: &str,
    ) -> Result<Box<dyn std::io::Write>, super::model::Error> {
        match &self.mode {
            WriteMode::Directory(out_dir) => {
                let file_name = Self::get_csv_file_name(out_type, name);
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
        print_render_table_to_csv(writer, table_model)
    }
}

enum ZipWriteMode {
    FilePath(PathBuf),
    Writer(WriteHandle),
}

pub struct CsvZipWriter {
    mode: ZipWriteMode,
    zip_writer: Option<crate::util::zip::ZipWriter>,
}

impl CsvZipWriter {
    pub fn new_to_file(zip_file_path: PathBuf) -> CsvZipWriter {
        CsvZipWriter {
            mode: ZipWriteMode::FilePath(zip_file_path),
            zip_writer: Some(crate::util::zip::ZipWriter::new()),
        }
    }

    pub fn new_to_writer(wh: WriteHandle) -> CsvZipWriter {
        CsvZipWriter {
            mode: ZipWriteMode::Writer(wh),
            zip_writer: Some(crate::util::zip::ZipWriter::new()),
        }
    }

    fn get_writer(
        &mut self,
    ) -> Result<Box<dyn std::io::Write>, super::model::Error> {
        match &self.mode {
            ZipWriteMode::FilePath(path) => {
                let file = File::create(path).map_err(|e| e.to_string())?;
                Ok(Box::new(file))
            }
            ZipWriteMode::Writer(write_handle) => Ok(Box::new(write_handle.clone())),
        }
    }
}

impl AcbWriter for CsvZipWriter {
    fn print_render_table(
        &mut self,
        out_type: OutputType,
        name: &str,
        table_model: &crate::portfolio::render::RenderTable,
    ) -> Result<(), super::model::Error> {
        if self.zip_writer.is_none() {
            return Err("Zip writer is not initialized".into());
        }

        let zip_writer = self.zip_writer.as_mut().unwrap();

        let file_name = CsvWriter::get_csv_file_name(out_type, name);
        let mut file_writer = zip_writer.start_file(&file_name)?;
        let mut data_writer = file_writer.create_data_writer();

        print_render_table_to_csv(&mut data_writer, table_model)?;
        let (_, descriptor) = data_writer
            .finish()
            .map_err(|e| format!("Failed to finish zip data writer: {}", e))?;
        file_writer.finish(descriptor)?;
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<(), super::model::Error> {
        let mut writer = self.get_writer()?;

        if self.zip_writer.is_none() {
            return Err("Zip writer is not initialized".into());
        }
        let zip_writer = self.zip_writer.take().unwrap();
        zip_writer.finish().and_then(|zip_data: Vec<u8>| {
            io::Write::write_all(&mut writer, &zip_data)
                .map_err(|e| format!("Failed to write zip data: {}", e))
        })?;
        Ok(())
    }
}
