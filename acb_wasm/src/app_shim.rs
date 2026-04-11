use std::collections::HashMap;

use serde::ser::SerializeStruct;

use acb::{
    app::{
        outfmt::text::TextWriter, run_acb_app_to_writer, AppRenderMode,
        AppSummaryRenderOutput, CalcRenderOutput,
    },
    portfolio::{io::tx_csv::TxCsvParseOptions, render::RenderTable, Security},
    util::{
        basic::SError,
        rw::{DescribedReader, WriteHandle},
    },
};

use crate::wasm_rates_loader::{
    build_rates_cache_update, make_rate_loader, RatesCacheData, RatesCacheUpdate,
};

const RENDER_TOTAL_COSTS: bool = false;

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
    pub rates_cache_update: RatesCacheUpdate,
}

impl serde::ser::Serialize for AppResultOk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // https://serde.rs/impl-serialize.html

        let n_fields = 3;
        let mut state = serializer.serialize_struct("AppResultOk", n_fields)?;
        state.serialize_field("textOutput", &self.text_output)?;
        state.serialize_field("modelOutput", &self.model_output)?;
        state.serialize_field("ratesCacheUpdate", &self.rates_cache_update)?;
        state.end()
    }
}

pub async fn run_acb_app(
    csv_file_readers: Vec<DescribedReader>,
    render_full_dollar_values: bool,
    initial_rates: Option<&RatesCacheData>,
) -> Result<AppResultOk, SError> {
    let (out_write_handle, out_string_buff) =
        WriteHandle::string_buff_write_handle();
    let (err_write_handle, err_string_buff) =
        WriteHandle::string_buff_write_handle();

    let writer = Box::new(TextWriter::new(out_write_handle));

    let mut rate_loader = make_rate_loader(err_write_handle.clone(), initial_rates)?;

    let result = run_acb_app_to_writer(
        writer,
        csv_file_readers,
        &TxCsvParseOptions::default(),
        None,
        None,
        render_full_dollar_values,
        RENDER_TOTAL_COSTS,
        AppRenderMode::Default,
        &mut rate_loader,
        err_write_handle,
    )
    .await;

    let rates_cache_update = build_rates_cache_update(&mut rate_loader);

    match result {
        Ok(r) => {
            let calc_variant = match r.output {
                CalcRenderOutput::Default(v) => v,
                CalcRenderOutput::ByAffiliate(v) => v.unfiltered,
            };
            Ok(AppResultOk {
                text_output: out_string_buff
                    .try_borrow_mut()
                    .unwrap()
                    .export_string(),
                model_output: AppRenderResult {
                    security_tables: calc_variant
                        .security_tables
                        .into_iter()
                        .map(|(k, v)| (k, SerializableRenderTable(v)))
                        .collect(),
                    aggregate_gains_table: SerializableRenderTable(
                        calc_variant.aggregate_gains_table,
                    ),
                },
                rates_cache_update,
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

pub struct AppExportResultOk {
    pub csv_files: Vec<FileContent>,
    pub rates_cache_update: RatesCacheUpdate,
}

impl serde::ser::Serialize for AppExportResultOk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // https://serde.rs/impl-serialize.html

        let n_fields = 2;
        let mut state =
            serializer.serialize_struct("AppExportResultOk", n_fields)?;
        state.serialize_field("csvFiles", &self.csv_files)?;
        state.serialize_field("ratesCacheUpdate", &self.rates_cache_update)?;
        state.end()
    }
}

pub async fn run_acb_app_for_export(
    csv_file_readers: Vec<DescribedReader>,
    render_full_dollar_values: bool,
    initial_rates: Option<&RatesCacheData>,
) -> Result<AppExportResultOk, SError> {
    let (err_write_handle, err_string_buff) =
        WriteHandle::string_buff_write_handle();

    let csv_coll = acb::util::rc::RcRefCellT::new(Vec::new());

    let writer = Box::new(acb::app::outfmt::csv::CsvWriter::new_to_collection(
        csv_coll.clone(),
    ));

    let mut rate_loader = make_rate_loader(err_write_handle.clone(), initial_rates)?;

    let result = run_acb_app_to_writer(
        writer,
        csv_file_readers,
        &TxCsvParseOptions::default(),
        None,
        None,
        render_full_dollar_values,
        RENDER_TOTAL_COSTS,
        AppRenderMode::ByAffiliateIfMultiple,
        &mut rate_loader,
        err_write_handle,
    )
    .await;

    let rates_cache_update = build_rates_cache_update(&mut rate_loader);

    match result {
        Ok(_) => Ok(AppExportResultOk {
            csv_files: csv_coll.take().into_iter().map(FileContent::from).collect(),
            rates_cache_update,
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

pub struct XlConvertResult {
    pub csv_text: String,
    pub non_fatal_errors: Vec<String>,
}

impl serde::ser::Serialize for XlConvertResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 2;
        let mut state = serializer.serialize_struct("XlConvertResult", n_fields)?;
        state.serialize_field("csvText", &self.csv_text)?;
        state.serialize_field("nonFatalErrors", &self.non_fatal_errors)?;
        state.end()
    }
}

pub struct CsvBrokerConvertResult {
    pub csv_text: String,
    pub non_fatal_errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl serde::ser::Serialize for CsvBrokerConvertResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 3;
        let mut state =
            serializer.serialize_struct("CsvBrokerConvertResult", n_fields)?;
        state.serialize_field("csvText", &self.csv_text)?;
        state.serialize_field("nonFatalErrors", &self.non_fatal_errors)?;
        state.serialize_field("warnings", &self.warnings)?;
        state.end()
    }
}

pub struct EtradeConvertResult {
    pub csv_text: String,
    pub warnings: Vec<String>,
}

impl serde::ser::Serialize for EtradeConvertResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 2;
        let mut state =
            serializer.serialize_struct("EtradeConvertResult", n_fields)?;
        state.serialize_field("csvText", &self.csv_text)?;
        state.serialize_field("warnings", &self.warnings)?;
        state.end()
    }
}

pub struct EtradeExtractResult {
    pub benefits_table: SerializableRenderTable,
    pub trades_table: SerializableRenderTable,
}

impl serde::ser::Serialize for EtradeExtractResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 2;
        let mut state =
            serializer.serialize_struct("EtradeExtractResult", n_fields)?;
        state.serialize_field("benefitsTable", &self.benefits_table)?;
        state.serialize_field("tradesTable", &self.trades_table)?;
        state.end()
    }
}

pub struct AppSummaryResultOk {
    pub csv_text: String,
    pub summary_table: SerializableRenderTable,
    pub rates_cache_update: RatesCacheUpdate,
}

impl serde::ser::Serialize for AppSummaryResultOk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let n_fields = 3;
        let mut state =
            serializer.serialize_struct("AppSummaryResultOk", n_fields)?;
        state.serialize_field("csvText", &self.csv_text)?;
        state.serialize_field("summaryTable", &self.summary_table)?;
        state.serialize_field("ratesCacheUpdate", &self.rates_cache_update)?;
        state.end()
    }
}

/// A serializable account extracted from a broker file, with the config key
/// identifying the broker and the raw account type/number.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct ExtractedAccount {
    /// The config key for the broker (e.g. "questrade", "rbc_di", "etrade").
    pub broker: String,
    /// The account number as a string.
    pub account_num: String,
    /// The broker-detected account type (e.g. "TFSA", "Individual margin").
    pub account_type: String,
}

impl ExtractedAccount {
    fn from_account(account: &acb::peripheral::broker::Account) -> Option<Self> {
        use acb::peripheral::broker::config_key_for_broker_name;
        let broker = config_key_for_broker_name(account.broker_name)?;
        if account.account_num.is_empty() {
            return None;
        }
        Some(ExtractedAccount {
            broker: broker.to_string(),
            account_num: account.account_num.clone(),
            account_type: account.account_type.clone(),
        })
    }
}

/// Result of extracting account numbers from broker files.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountExtractionResult {
    pub accounts: Vec<ExtractedAccount>,
    pub warnings: Vec<String>,
}

/// Convert raw broker `Account` objects into a deduplicated, sorted, serializable result.
pub fn to_account_extraction_result(
    accounts: Vec<acb::peripheral::broker::Account>,
    warnings: Vec<String>,
) -> AccountExtractionResult {
    use std::collections::HashSet;

    let mut seen = HashSet::<ExtractedAccount>::new();
    for account in &accounts {
        if let Some(ea) = ExtractedAccount::from_account(account) {
            seen.insert(ea);
        }
    }

    let mut extracted: Vec<ExtractedAccount> = seen.into_iter().collect();
    extracted.sort_by(|a, b| {
        a.broker.cmp(&b.broker).then_with(|| a.account_num.cmp(&b.account_num))
    });

    AccountExtractionResult {
        accounts: extracted,
        warnings,
    }
}

pub async fn run_acb_app_summary(
    latest_date: acb::util::date::Date,
    csv_file_readers: Vec<DescribedReader>,
    split_annual_summary_gains: bool,
    render_full_dollar_values: bool,
    initial_rates: Option<&RatesCacheData>,
) -> Result<AppSummaryResultOk, SError> {
    use acb::app::{run_acb_app_summary_to_render_model, Options};

    let (err_write_handle, err_string_buff) =
        WriteHandle::string_buff_write_handle();

    let mut rate_loader = make_rate_loader(err_write_handle.clone(), initial_rates)?;

    let options = Options {
        render_full_dollar_values,
        split_annual_summary_gains,
        ..Options::default()
    };

    let result = run_acb_app_summary_to_render_model(
        latest_date,
        csv_file_readers,
        options,
        None,
        &mut rate_loader,
        err_write_handle,
    )
    .await;

    let rates_cache_update = build_rates_cache_update(&mut rate_loader);

    let buffered_error_string =
        err_string_buff.try_borrow_mut().unwrap().export_string();
    let mut errors: Vec<String> = vec![];
    if !buffered_error_string.is_empty() {
        // Most of the time these are warnings, so put them first.
        // Other times they /could/ be duplicates (though unlikely),
        // but if that were the case, they'd naturally be printed first
        // and wrapped into an error second, so we should render them
        // in that order.
        errors.push(buffered_error_string);
    }
    errors.extend(result.errors.iter().map(String::clone));

    let (summary_txs, csv_text) = match result.output {
        AppSummaryRenderOutput::Default(v) => (v.summary_txs, v.csv_text),
        AppSummaryRenderOutput::ByAffiliate(v) => {
            (v.unfiltered.summary_txs, v.unfiltered.csv_text)
        }
    };

    let csv_txs: Vec<acb::portfolio::CsvTx> = summary_txs
        .iter()
        .map(|tx| acb::portfolio::CsvTx::from(tx.clone()))
        .collect();
    let table = acb::portfolio::io::tx_csv::txs_to_csv_table(&csv_txs);
    let summary_table = SerializableRenderTable(RenderTable {
        header: table.header.into_iter().map(|s| s.to_string()).collect(),
        rows: table.rows,
        footer: vec![],
        notes: vec![],
        errors,
    });

    Ok(AppSummaryResultOk {
        csv_text,
        summary_table,
        rates_cache_update,
    })
}
