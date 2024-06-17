use std::io::Write;

fn main() {
    if acb::peripheral::tx_export_convert_impl::run().is_err() {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        std::process::exit(1);
    }
}