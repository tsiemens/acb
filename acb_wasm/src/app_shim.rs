use std::collections::HashMap;

use serde::ser::SerializeStruct;

use acb::{
    app::{outfmt::text::TextWriter, run_acb_app_to_writer},
    fx::io::{InMemoryRatesCache, JsonRemoteRateLoader, RateLoader},
    portfolio::{
        io::tx_csv::TxCsvParseOptions, render::RenderTable, PortfolioSecurityStatus,
        Security,
    },
    util::{
        basic::SError,
        rw::{DescribedReader, WriteHandle},
    },
};

const FORCE_DOWNLOAD_RATES: bool = false;
const RENDER_TOTAL_COSTS: bool = false;

fn make_rate_loader(err_write_handle: WriteHandle) -> RateLoader {
    RateLoader::new(
        FORCE_DOWNLOAD_RATES,
        Box::new(InMemoryRatesCache::new()),
        JsonRemoteRateLoader::new_boxed(
            crate::http::CorsEnabledHttpRequester::new_boxed(),
        ),
        err_write_handle,
    )
}

pub struct SerializableRenderTable(pub RenderTable);

impl serde::ser::Serialize for SerializableRenderTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 5;
        let mut state =
            serializer.serialize_struct("SerializableRenderTable", n_fields)?;
        state.serialize_field("header", &self.0.header)?;
        state.serialize_field("rows", &self.0.rows)?;
        state.serialize_field("footer", &self.0.footer)?;
        state.serialize_field("notes", &self.0.notes)?;
        state.serialize_field("errors", &self.0.errors)?;
        state.end()
    }
}

pub struct AppRenderResult {
    pub security_tables: HashMap<Security, SerializableRenderTable>,
    pub aggregate_gains_table: SerializableRenderTable,
}

impl serde::ser::Serialize for AppRenderResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 2;
        let mut state = serializer.serialize_struct("AppRenderResult", n_fields)?;
        state.serialize_field("securityTables", &self.security_tables)?;
        state.serialize_field("aggregateGainsTable", &self.aggregate_gains_table)?;
        state.end()
    }
}

pub struct AppResultOk {
    pub text_output: String,
    pub model_output: AppRenderResult,
}

impl serde::ser::Serialize for AppResultOk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // https://serde.rs/impl-serialize.html

        let n_fields = 2;
        let mut state = serializer.serialize_struct("AppResultOk", n_fields)?;
        state.serialize_field("textOutput", &self.text_output)?;
        state.serialize_field("modelOutput", &self.model_output)?;
        state.end()
    }
}

pub async fn run_acb_app(
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    render_full_dollar_values: bool,
) -> Result<AppResultOk, SError> {
    let (out_write_handle, out_string_buff) =
        WriteHandle::string_buff_write_handle();
    let (err_write_handle, err_string_buff) =
        WriteHandle::string_buff_write_handle();

    let writer = Box::new(TextWriter::new(out_write_handle));

    let rate_loader = make_rate_loader(err_write_handle.clone());

    let result = run_acb_app_to_writer(
        writer,
        csv_file_readers,
        all_init_status,
        &TxCsvParseOptions::default(),
        render_full_dollar_values,
        RENDER_TOTAL_COSTS,
        rate_loader,
        err_write_handle,
    )
    .await;

    match result {
        Ok(r) => Ok(AppResultOk {
            text_output: out_string_buff.try_borrow_mut().unwrap().export_string(),
            model_output: AppRenderResult {
                security_tables: r
                    .security_tables
                    .into_iter()
                    .map(|(k, v)| (k, SerializableRenderTable(v)))
                    .collect(),
                aggregate_gains_table: SerializableRenderTable(
                    r.aggregate_gains_table,
                ),
            },
        }),
        Err(()) => {
            let error_string =
                err_string_buff.try_borrow_mut().unwrap().export_string();
            if !error_string.is_empty() {
                Err(error_string)
            } else {
                Err("Unknown error".to_string())
            }
        }
    }
}

pub struct FileContent {
    file_name: String,
    content: String,
}

impl From<acb::app::outfmt::csv::Utf8FileContent> for FileContent {
    fn from(value: acb::app::outfmt::csv::Utf8FileContent) -> Self {
        FileContent {
            file_name: value.file_name,
            content: value.content,
        }
    }
}

impl serde::ser::Serialize for FileContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 2;
        let mut state = serializer.serialize_struct("FileTContent", n_fields)?;
        state.serialize_field("fileName", &self.file_name)?;
        state.serialize_field("content", &self.content)?;
        state.end()
    }
}

// TODO move this to a separate file
pub struct AppExportResultOk {
    pub csv_files: Vec<FileContent>,
}

impl serde::ser::Serialize for AppExportResultOk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // https://serde.rs/impl-serialize.html

        let n_fields = 1;
        let mut state = serializer.serialize_struct("AppExportResultOk", n_fields)?;
        state.serialize_field("csvFiles", &self.csv_files)?;
        state.end()
    }
}

pub async fn run_acb_app_for_export(
    csv_file_readers: Vec<DescribedReader>,
    all_init_status: HashMap<Security, PortfolioSecurityStatus>,
    render_full_dollar_values: bool,
) -> Result<AppExportResultOk, SError> {
    let (err_write_handle, err_string_buff) =
        WriteHandle::string_buff_write_handle();

    let csv_coll = acb::util::rc::RcRefCellT::new(Vec::new());

    let writer = Box::new(acb::app::outfmt::csv::CsvWriter::new_to_collection(
        csv_coll.clone()));

    let rate_loader = make_rate_loader(err_write_handle.clone());

    let result = run_acb_app_to_writer(
        writer,
        csv_file_readers,
        all_init_status,
        &TxCsvParseOptions::default(),
        render_full_dollar_values,
        RENDER_TOTAL_COSTS,
        rate_loader,
        err_write_handle,
    )
    .await;

    match result {
        Ok(_) => {
            Ok(AppExportResultOk {
                csv_files: csv_coll.take().into_iter()
                    .map(FileContent::from)
                    .collect(),
            })
        }
        Err(()) => {
            let error_string =
                err_string_buff.try_borrow_mut().unwrap().export_string();
            if !error_string.is_empty() {
                Err(error_string)
            } else {
                Err("Unknown error".to_string())
            }
        }
    }
}
