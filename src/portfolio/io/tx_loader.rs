use rust_decimal::Decimal;
use time::Date;

use crate::{fx::io::RateLoader, portfolio::{CsvTx, Currency}};

type Error = String;

async fn load_rate_if_needed(
    trade_date: &Date,
    curr: &Option<Currency>, provided_rate: &Option<Decimal>,
    rate_loader: &mut RateLoader,
    ) -> Result<Option<Decimal>, Error> {
    if provided_rate.is_some() {
        return Ok(None)
    }
    match curr {
        Some(c) => {
            if c.is_default() {
                return Ok(None)
            }
            if *c != Currency::usd() {
                return Err(format!(
                    "Currency {} does not support automatically loaded day rates. Rate must be provided.",
                    *c))
            }
        },
        None => {
            // Treated as default (CAD)
            return Ok(None)
        }
    }
    // Rate is USD, and needs a rate loaded
    let rate = rate_loader.get_effective_usd_cad_rate(trade_date.clone()).await?;

    Ok(Some(rate.foreign_to_local_rate))
}

// Takes CsvTxs parsed from a CSV, and loads any missing exchange
// rates directly into them.
// This will be dependent on if any currency is set to USD, but the rate
// is None.
pub async fn load_tx_rates(
    csv_txs: &mut Vec<CsvTx>,
    rate_loader: &mut RateLoader,
    ) -> Result<(), Error> {

    for tx in csv_txs {
        let trade_date = tx.trade_date.as_ref().ok_or("Tx has no trade date")?;
        let loaded_rate = load_rate_if_needed(
            trade_date,
            &tx.tx_currency, &tx.tx_curr_to_local_exchange_rate,
            rate_loader).await.map_err(|e| format!(
                "Exchange rate error: {}", e))?;

        if let Some(r) = loaded_rate {
            tx.tx_curr_to_local_exchange_rate = Some(r);
        }

        let c_loaded_rate = load_rate_if_needed(
            trade_date,
            &tx.commission_currency, &tx.commission_curr_to_local_exchange_rate,
            rate_loader).await.map_err(|e| format!(
                "Commission exchange rate error: {}", e))?;

        if let Some(r) = c_loaded_rate {
            tx.commission_curr_to_local_exchange_rate = Some(r);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::Date;

    use crate::{fx::{io::testlib::new_test_rate_loader, DailyRate}, portfolio::{io::tx_loader::load_tx_rates, CsvTx, Currency}, testlib::assert_re, tracing::setup_tracing, util::date::pub_testlib::doy_date};

    fn odate_yd(year: u32, doy: i64) -> Option<Date> {
        Some(doy_date(year, doy))
    }

    fn dr(date: Date, rate: Decimal) -> DailyRate {
        DailyRate{date: date, foreign_to_local_rate: rate}
    }

    #[test]
    fn test_load_tx_rates() {
        setup_tracing();

        crate::util::date::set_todays_date_for_test(doy_date(2024, 150));

        let (mut rate_loader, _, remote_rates) =
            new_test_rate_loader(false);
        remote_rates.borrow_mut().insert(2024, vec![
            dr(doy_date(2024, 10), dec!(1.2)),
        ]);

        let usd = Some(Currency::usd());
        let cad = Some(Currency::cad());

        // Ok cases
        let mut csv_txs = vec![
            // USD Tx without any rates.
            // Date after available remote rate (proves use effective)
            CsvTx{trade_date: odate_yd(2024, 11),
                  tx_currency: usd.clone(), commission_currency: usd.clone(),
                  ..CsvTx::default()},
            // CAD
            CsvTx{trade_date: odate_yd(2024, 11),
                  tx_currency: cad.clone(), commission_currency: cad.clone(),
                  ..CsvTx::default()},
            // Unspecified (defaults to CAD)
            CsvTx{trade_date: odate_yd(2024, 11),
                  ..CsvTx::default()},
            // Mixed
            CsvTx{trade_date: odate_yd(2024, 11),
                  tx_currency: usd.clone(), commission_currency: cad.clone(),
                  ..CsvTx::default()},
            CsvTx{trade_date: odate_yd(2024, 11),
                  commission_currency: usd.clone(),
                  ..CsvTx::default()},
            // Mixed explicit
            CsvTx{trade_date: odate_yd(2024, 11),
                tx_currency: usd.clone(), tx_curr_to_local_exchange_rate: Some(dec!(1.6)),
                commission_currency: usd.clone(),
                ..CsvTx::default()},
            CsvTx{trade_date: odate_yd(2024, 11),
                tx_currency: usd.clone(),
                commission_currency: usd.clone(), commission_curr_to_local_exchange_rate: Some(dec!(1.5)),
                ..CsvTx::default()},
            // Explicit (with un-fetchable date)
            CsvTx{trade_date: odate_yd(2024, 300),
                tx_currency: usd.clone(), tx_curr_to_local_exchange_rate: Some(dec!(1.7)),
                commission_currency: usd.clone(), commission_curr_to_local_exchange_rate: Some(dec!(1.8)),
                ..CsvTx::default()},
        ];

        async_std::task::block_on(load_tx_rates(&mut csv_txs, &mut rate_loader)).unwrap();
        // USD
        assert_eq!(csv_txs[0].tx_curr_to_local_exchange_rate, Some(dec!(1.2)));
        assert_eq!(csv_txs[0].commission_curr_to_local_exchange_rate, Some(dec!(1.2)));
        // CAD
        assert_eq!(csv_txs[1].tx_curr_to_local_exchange_rate, None);
        assert_eq!(csv_txs[1].commission_curr_to_local_exchange_rate, None);

        // Unspecified
        assert_eq!(csv_txs[2].tx_curr_to_local_exchange_rate, None);
        assert_eq!(csv_txs[2].commission_curr_to_local_exchange_rate, None);

        // Mixed
        assert_eq!(csv_txs[3].tx_curr_to_local_exchange_rate, Some(dec!(1.2)));
        assert_eq!(csv_txs[3].commission_curr_to_local_exchange_rate, None);

        assert_eq!(csv_txs[4].tx_curr_to_local_exchange_rate, None);
        assert_eq!(csv_txs[4].commission_curr_to_local_exchange_rate, Some(dec!(1.2)));

        // Mixed explicit
        assert_eq!(csv_txs[5].tx_curr_to_local_exchange_rate, Some(dec!(1.6)));
        assert_eq!(csv_txs[5].commission_curr_to_local_exchange_rate, Some(dec!(1.2)));

        assert_eq!(csv_txs[6].tx_curr_to_local_exchange_rate, Some(dec!(1.2)));
        assert_eq!(csv_txs[6].commission_curr_to_local_exchange_rate, Some(dec!(1.5)));

        // Explicit
        assert_eq!(csv_txs[7].tx_curr_to_local_exchange_rate, Some(dec!(1.7)));
        assert_eq!(csv_txs[7].commission_curr_to_local_exchange_rate, Some(dec!(1.8)));
    }

    #[test]
    fn test_load_tx_rates_errors() {
        setup_tracing();

        crate::util::date::set_todays_date_for_test(doy_date(2024, 100));

        let do_error_test = |csv_tx: CsvTx| {
            let (mut rate_loader, _, remote_rates) =
                new_test_rate_loader(false);
            remote_rates.borrow_mut().insert(2024, vec![
                dr(doy_date(2024, 10), dec!(1.2)),
            ]);

            let mut csv_txs = vec![csv_tx];
            async_std::task::block_on(load_tx_rates(&mut csv_txs, &mut rate_loader)).unwrap_err()
        };

        let usd = Some(Currency::usd());

        let err = do_error_test(CsvTx::default());
        assert_eq!(err, "Tx has no trade date");

        // No rate that can be found (after current date)
        let err = do_error_test(
            CsvTx{trade_date: odate_yd(2024, 150),
                tx_currency: usd.clone(),
                ..CsvTx::default()});
        assert_re(r"^Exchange rate error: Unable to retrieve exchange rate for 2024-05-30", &err);

        // No commission rate that can be found (after current date)
        let err = do_error_test(
            CsvTx{trade_date: odate_yd(2024, 150),
                commission_currency: usd.clone(),
                ..CsvTx::default()});
        assert_re(r"^Commission exchange rate error: Unable to retrieve exchange rate for 2024-05-30", &err);
    }
}