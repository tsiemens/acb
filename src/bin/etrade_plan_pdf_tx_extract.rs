use std::io::Write;

fn main() {
    if acb::peripheral::etrade_plan_pdf_tx_extract_impl::run().is_err() {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        std::process::exit(1);
    }
}
