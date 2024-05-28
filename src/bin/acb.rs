fn main() {
    if let Err(_) = acb::cmd::command_main() {
        // The code itself is currently unstable, so we
        // cant yet call exit_process or to_i32.
        std::process::exit(1);
    }
}
