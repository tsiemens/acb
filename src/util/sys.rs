pub fn env_var_non_empty(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => !v.is_empty(),
        Err(_) => false,
    }
}