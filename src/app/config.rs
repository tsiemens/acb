use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::util::basic::SError;

const CURRENT_CONFIG_VERSION: u32 = 1;

/// Top-level ACB configuration.
///
/// The config file is always pretty-printed JSON.  Unknown top-level
/// fields are silently ignored on deserialization so that an older acb
/// can still read a config written by a newer version (as long as the
/// `version` is still supported).
fn default_version() -> u32 {
    CURRENT_CONFIG_VERSION
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AcbConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub account_bindings: AccountBindings,
    /// Global symbol rename map: `from → to`.
    ///
    /// Applied at every ingestion point (broker import and ACB calculation).
    /// Single-pass, no chaining: `A → B` and `B → C` will rename `A` to `B`
    /// only; `B` to `C` only. Case-sensitive exact match.
    #[serde(default)]
    pub symbol_renames: HashMap<String, String>,
}

/// Per-broker maps of `account_num → affiliate_name`.
///
/// Broker keys: `"questrade"`, `"rbc_di"`, `"etrade"`.
/// Affiliate name is the display name only (e.g. `"Spouse"`);
/// registered status is always auto-detected from account type.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct AccountBindings {
    #[serde(default)]
    pub questrade: HashMap<String, String>,
    #[serde(default)]
    pub rbc_di: HashMap<String, String>,
    #[serde(default)]
    pub etrade: HashMap<String, String>,
}

impl AcbConfig {
    pub fn new() -> Self {
        AcbConfig {
            version: CURRENT_CONFIG_VERSION,
            account_bindings: AccountBindings::default(),
            symbol_renames: HashMap::new(),
        }
    }

    /// Deserialize from JSON, validating the version.
    pub fn from_json(json: &str) -> Result<Self, SError> {
        // First do a partial parse to check version before full deser.
        let raw: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| format!("Invalid config JSON: {e}"))?;

        let version =
            raw.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        if version > CURRENT_CONFIG_VERSION {
            return Err(format!(
                "Config file version {version} is newer than the supported \
                 version ({CURRENT_CONFIG_VERSION}). Please upgrade acb."
            ));
        }

        let config: AcbConfig = serde_json::from_value(raw)
            .map_err(|e| format!("Failed to parse config: {e}"))?;

        Ok(config)
    }

    /// Serialize to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, SError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))
    }

    /// Look up an affiliate name for the given broker key and account number.
    /// Returns `None` if no binding exists.
    pub fn affiliate_name_for_account(
        &self,
        broker_key: &str,
        account_num: &str,
    ) -> Option<&str> {
        let map = match broker_key {
            "questrade" => &self.account_bindings.questrade,
            "rbc_di" => &self.account_bindings.rbc_di,
            "etrade" => &self.account_bindings.etrade,
            _ => return None,
        };
        map.get(account_num).map(|s| s.as_str())
    }

    /// Return warnings for unrecognized broker keys (future-proofing helper).
    /// Since we use typed fields, this is only possible if someone manually
    /// adds extra keys.  We check via the raw JSON value.
    pub fn validate_warnings(json: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        let known_keys: &[&str] = &["version", "account_bindings", "symbol_renames"];
        let known_broker_keys: &[&str] = &["questrade", "rbc_di", "etrade"];

        if let Ok(raw) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(obj) = raw.as_object() {
                for key in obj.keys() {
                    if !known_keys.contains(&key.as_str()) {
                        warnings.push(format!(
                            "Unknown top-level config key: \"{key}\""
                        ));
                    }
                }
            }
            if let Some(bindings) = raw.get("account_bindings") {
                if let Some(obj) = bindings.as_object() {
                    for key in obj.keys() {
                        if !known_broker_keys.contains(&key.as_str()) {
                            warnings.push(format!(
                                "Unknown broker key in account_bindings: \"{key}\""
                            ));
                        }
                    }
                }
            }
        }
        warnings
    }
}

/// Look up `sym` in the config's `symbol_renames` map.
///
/// Returns the renamed symbol if a mapping exists, otherwise returns `sym`
/// unchanged. Single-pass: no chaining is performed.
pub fn rename_symbol<'a>(config: &'a AcbConfig, sym: &'a str) -> &'a str {
    config.symbol_renames.get(sym).map(|s| s.as_str()).unwrap_or(sym)
}

/// Default config file path: `~/.config/acb/acb-config.json`
pub fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("acb").join("acb-config.json"))
}

/// Load config from a path, returning `None` if the file doesn't exist.
pub fn load_config(path: &std::path::Path) -> Result<Option<AcbConfig>, SError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let warnings = AcbConfig::validate_warnings(&contents);
            for w in &warnings {
                eprintln!("Config warning: {w}");
            }
            Ok(Some(AcbConfig::from_json(&contents)?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!(
            "Failed to read config file {}: {e}",
            path.display()
        )),
    }
}

/// Save config to a path, creating parent directories if needed.
pub fn save_config(
    config: &AcbConfig,
    path: &std::path::Path,
) -> Result<(), SError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }
    let json = config.to_json()?;
    std::fs::write(path, json)
        .map_err(|e| format!("Failed to write config file: {e}"))
}

// Broker key constants used in config lookups.
// These map broker_name constants to config keys.
pub const QUESTRADE_CONFIG_KEY: &str = "questrade";
pub const RBC_DI_CONFIG_KEY: &str = "rbc_di";
pub const ETRADE_CONFIG_KEY: &str = "etrade";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut config = AcbConfig::new();
        config
            .account_bindings
            .questrade
            .insert("12345".to_string(), "Spouse".to_string());
        config
            .account_bindings
            .rbc_di
            .insert("99999".to_string(), "Spouse".to_string());

        let json = config.to_json().unwrap();
        let parsed = AcbConfig::from_json(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_minimal_config() {
        let json = r#"{ "version": 1 }"#;
        let config = AcbConfig::from_json(json).unwrap();
        assert_eq!(config.version, 1);
        assert!(config.account_bindings.questrade.is_empty());
    }

    #[test]
    fn test_missing_version_defaults_to_1() {
        let json = r#"{ "account_bindings": {} }"#;
        let config = AcbConfig::from_json(json).unwrap();
        assert_eq!(config.version, 1);
    }

    #[test]
    fn test_unsupported_version() {
        let json = r#"{ "version": 999 }"#;
        let err = AcbConfig::from_json(json).unwrap_err();
        assert!(err.contains("newer than the supported version"));
        assert!(err.contains("upgrade acb"));
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let json = r#"{
            "version": 1,
            "account_bindings": {
                "questrade": { "12345": "Spouse" }
            },
            "some_future_field": true
        }"#;
        let config = AcbConfig::from_json(json).unwrap();
        assert_eq!(
            config.affiliate_name_for_account("questrade", "12345"),
            Some("Spouse")
        );
    }

    #[test]
    fn test_validate_warnings() {
        let json = r#"{
            "version": 1,
            "account_bindings": {
                "questrade": {},
                "unknown_broker": {}
            },
            "mystery": 42
        }"#;
        let warnings = AcbConfig::validate_warnings(json);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("mystery"));
        assert!(warnings[1].contains("unknown_broker"));
    }

    #[test]
    fn test_round_trip_with_symbol_renames() {
        let mut config = AcbConfig::new();
        config.symbol_renames.insert("XEQT".to_string(), "XEQT.TO".to_string());
        config.symbol_renames.insert("VFV".to_string(), "VFV.TO".to_string());

        let json = config.to_json().unwrap();
        let parsed = AcbConfig::from_json(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_rename_symbol() {
        let mut config = AcbConfig::new();
        config.symbol_renames.insert("XEQT".to_string(), "XEQT.TO".to_string());

        assert_eq!(rename_symbol(&config, "XEQT"), "XEQT.TO");
        assert_eq!(rename_symbol(&config, "XEQT.TO"), "XEQT.TO");
        assert_eq!(rename_symbol(&config, "VFV"), "VFV");
    }

    #[test]
    fn test_rename_symbol_no_chaining() {
        let mut config = AcbConfig::new();
        config.symbol_renames.insert("A".to_string(), "B".to_string());
        config.symbol_renames.insert("B".to_string(), "C".to_string());

        // Single-pass: A → B (stops there, does not chain to C)
        assert_eq!(rename_symbol(&config, "A"), "B");
        // Direct hit: B → C
        assert_eq!(rename_symbol(&config, "B"), "C");
    }

    #[test]
    fn test_affiliate_name_lookup() {
        let mut config = AcbConfig::new();
        config
            .account_bindings
            .etrade
            .insert("XXXX-1234".to_string(), "Spouse".to_string());

        assert_eq!(
            config.affiliate_name_for_account("etrade", "XXXX-1234"),
            Some("Spouse")
        );
        assert_eq!(
            config.affiliate_name_for_account("etrade", "XXXX-9999"),
            None
        );
        assert_eq!(
            config.affiliate_name_for_account("unknown", "XXXX-1234"),
            None
        );
    }
}
