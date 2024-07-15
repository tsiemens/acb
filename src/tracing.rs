use time::format_description;
use tracing_subscriber::{fmt, EnvFilter, FmtSubscriber};

// Sets up tracing. Goes to stderr, filtered by TRACE env var.
// Levels are: trace, debug, info, warn, error
//
// EnvFilter has a standard syntax, but basically can be boiled down to (for example):
//
// All targets, info level:             info
// All modules under fx, debug level:   acb::fx=debug
// Global at info, fx as debug:         info,acb::fx=debug
//
// More generally: target[span{field=value}]=level
// https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html
pub fn setup_tracing() {
    // Define the time format. 5 digits of precision is apparently good enough.
    let time_format =
        format_description::parse("[hour]:[minute]:[second].[subsecond digits:5]")
            .expect("Time format description is invalid");

    let time_offset = crate::util::date::local_utc_offset().unwrap();
    let timer = fmt::time::OffsetTime::new(time_offset, time_format);

    // Create a subscriber that uses stderr for tracing.
    // It will use the TRACE env var for filtering, and is off by default
    let subscriber = FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_env("TRACE"))
        .with_timer(timer) // Use custom time formatting
        .finish();

    // Set the subscriber as the default
    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub fn enable_trace_env(trade_env: &str) {
    const VAR_NAME: &str = "TRACE";
    if let Ok(existing_env) = std::env::var(VAR_NAME) {
        std::env::set_var(VAR_NAME, existing_env + "," + trade_env);
    } else {
        std::env::set_var(VAR_NAME, trade_env);
    }
}
