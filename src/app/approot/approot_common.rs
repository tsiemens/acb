use std::collections::HashMap;

use time::Date;

use crate::{
    app::config::AcbConfig,
    fx::io::RateLoader,
    portfolio::{
        bookkeeping::{txs_to_delta_list, DeltaListResult},
        io::{
            tx_csv::{parse_tx_csv, TxCsvParseOptions},
            tx_loader::load_tx_rates,
        },
        tx_utils::apply_security_rename,
        AffiliateFilter, Security, Tx,
    },
    util::rw::{DescribedReader, WriteHandle},
};

pub type Error = String;

pub struct Options {
    pub affiliate_render_filter: Option<AffiliateFilter>,
    pub render_full_dollar_values: bool,
    pub summary_mode_latest_date: Option<Date>,
    pub split_annual_summary_gains: bool,
    pub render_total_costs: bool,
    pub csv_output_dir: Option<String>,
    pub csv_output_zip: Option<String>,
    pub csv_parse_options: TxCsvParseOptions,
}

impl Options {
    pub fn summary_mode(&self) -> bool {
        self.summary_mode_latest_date.is_some()
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            affiliate_render_filter: None,
            render_full_dollar_values: false,
            summary_mode_latest_date: None,
            split_annual_summary_gains: false,
            render_total_costs: false,
            csv_output_dir: None,
            csv_output_zip: None,
            csv_parse_options: TxCsvParseOptions::default(),
        }
    }
}

pub enum AppRenderMode {
    Default,
    ByAffiliateIfMultiple,
}

/// This is a partial component of the app as a whole, just to generate TxDeltas.
/// What this does _not_ do is do any aggregation calculations, like
/// yearly capital gains and costs.
pub async fn run_acb_app_to_delta_models(
    csv_file_readers: Vec<DescribedReader>,
    csv_parse_options: &TxCsvParseOptions,
    config: Option<&AcbConfig>,
    rate_loader: &mut RateLoader,
    mut err_printer: WriteHandle,
) -> Result<HashMap<Security, DeltaListResult>, Error> {
    let mut all_txs = Vec::<Tx>::new();
    let mut global_read_index: u32 = 0;
    for mut csv_reader in csv_file_readers {
        let mut csv_txs = parse_tx_csv(
            &mut csv_reader,
            global_read_index,
            &csv_parse_options,
            &mut err_printer,
        )?;

        if let Some(cfg) = config {
            for tx in &mut csv_txs {
                apply_security_rename(tx, &cfg.symbol_renames);
            }
        }

        load_tx_rates(&mut csv_txs, rate_loader).await?;

        let mut txs = Vec::<Tx>::with_capacity(csv_txs.len());
        for csv_tx in csv_txs {
            txs.push(Tx::try_from(csv_tx)?)
        }

        global_read_index += txs.len() as u32;
        all_txs.append(&mut txs);
    }

    all_txs.sort();
    let txs_by_sec = crate::portfolio::split_txs_by_security(all_txs);

    let mut delta_results = HashMap::<Security, DeltaListResult>::new();

    for (sec, mut sec_txs) in txs_by_sec {
        crate::portfolio::global_affiliate_txs::replace_global_security_txs(
            &mut sec_txs,
        )?;

        let deltas_res = txs_to_delta_list(&sec_txs);
        delta_results.insert(sec, deltas_res);
    }

    Ok(delta_results)
}
