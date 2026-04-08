use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::app::config::{
    self, default_config_path, load_config, save_config, AcbConfig,
};
use crate::util::basic::SError;

#[derive(clap::Args, Debug)]
struct ShowArgs {}

#[derive(clap::Args, Debug)]
struct PathArgs {}

#[derive(clap::Args, Debug)]
struct SetArgs {
    /// The config to set
    #[arg(required = true)]
    setting: String,

    /// Key-value equals-separated pairs to set for the config.
    ///
    /// Settings:
    ///
    /// - affiliate-account broker=questrade|rbc_di|etrade account=12345 affiliate=Spouse
    #[arg(required = true)]
    pub values: Vec<String>,
}

#[derive(clap::Args, Debug)]
struct UnsetArgs {
    /// The config to unset
    #[arg(required = true)]
    setting: String,

    /// Key-value pairs to identify sub-values to unset for the config.
    ///
    /// Typically this will mean all of the "identifying" attributes of a
    /// config entry.
    ///
    /// eg. affiliate-account broker=questrade account=12345
    #[arg(required = false)]
    pub values: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show the current config
    Show(ShowArgs),

    /// Emit the current config path
    Path(PathArgs),

    /// Set a config value
    Set(SetArgs),

    /// Remove a config value
    Unset(UnsetArgs),
}

/// A tool to update the common ACB user config file.
#[derive(Parser, Debug)]
#[command(author, about, long_about)]
struct Args {
    #[command(subcommand)]
    pub command: Command,

    /// Path to acb-config.json. Defaults to the platform config dir.
    ///
    ///   Linux:   ~/.config/acb/acb-config.json
    ///
    ///   macOS:   ~/Library/Application Support/acb/acb-config.json
    ///
    ///   Windows: %APPDATA%\acb\acb-config.json
    #[arg(long)]
    pub config: Option<PathBuf>,
}

const KNOWN_BROKERS: &[&str] = &[
    config::QUESTRADE_CONFIG_KEY,
    config::RBC_DI_CONFIG_KEY,
    config::ETRADE_CONFIG_KEY,
];

fn parse_kv_pairs(values: &[String]) -> Result<HashMap<String, String>, SError> {
    let mut map = HashMap::new();
    for v in values {
        let (key, val) = v
            .split_once('=')
            .ok_or_else(|| format!("Expected key=value, got: \"{v}\""))?;
        if key.is_empty() {
            return Err(format!("Empty key in: \"{v}\""));
        }
        map.insert(key.to_string(), val.to_string());
    }
    Ok(map)
}

fn require_key<'a>(
    kv: &'a HashMap<String, String>,
    key: &str,
) -> Result<&'a str, SError> {
    kv.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| format!("Missing required key: \"{key}\""))
}

fn broker_map_mut<'a>(
    config: &'a mut AcbConfig,
    broker: &str,
) -> Result<&'a mut HashMap<String, String>, SError> {
    match broker {
        "questrade" => Ok(&mut config.account_bindings.questrade),
        "rbc_di" => Ok(&mut config.account_bindings.rbc_di),
        "etrade" => Ok(&mut config.account_bindings.etrade),
        _ => Err(format!(
            "Unknown broker: \"{broker}\". Known brokers: {}",
            KNOWN_BROKERS.join(", ")
        )),
    }
}

/// Apply a "set" operation to the config. Returns a description of
/// what was changed.
pub fn apply_set(
    config: &mut AcbConfig,
    setting: &str,
    values: &[String],
) -> Result<String, SError> {
    match setting {
        "affiliate-account" => {
            let kv = parse_kv_pairs(values)?;
            let broker = require_key(&kv, "broker")?;
            let account = require_key(&kv, "account")?;
            let affiliate = require_key(&kv, "affiliate")?;

            let map = broker_map_mut(config, broker)?;
            map.insert(account.to_string(), affiliate.to_string());

            Ok(format!(
                "Set affiliate-account: broker={broker} \
                 account={account} affiliate={affiliate}"
            ))
        }
        _ => Err(format!(
            "Unknown setting: \"{setting}\". Known settings: affiliate-account"
        )),
    }
}

/// Apply an "unset" operation to the config. Returns a description of
/// what was removed, or an error if the entry was not found.
pub fn apply_unset(
    config: &mut AcbConfig,
    setting: &str,
    values: &[String],
) -> Result<String, SError> {
    match setting {
        "affiliate-account" => {
            let kv = parse_kv_pairs(values)?;
            let broker = require_key(&kv, "broker")?;
            let account = require_key(&kv, "account")?;

            let map = broker_map_mut(config, broker)?;
            if map.remove(account).is_some() {
                Ok(format!(
                    "Removed affiliate-account: broker={broker} account={account}"
                ))
            } else {
                Err(format!(
                    "No affiliate-account binding found for \
                     broker={broker} account={account}"
                ))
            }
        }
        _ => Err(format!(
            "Unknown setting: \"{setting}\". Known settings: affiliate-account"
        )),
    }
}

fn resolve_config_path(explicit: Option<&PathBuf>) -> Result<PathBuf, SError> {
    match explicit {
        Some(p) => Ok(p.clone()),
        None => default_config_path()
            .ok_or_else(|| "Could not determine config directory".to_string()),
    }
}

pub fn run() -> Result<(), ()> {
    let args = Args::parse();
    run_with_args(args).map_err(|e| {
        eprintln!("Error: {e}");
    })
}

fn run_with_args(args: Args) -> Result<(), SError> {
    let config_path = resolve_config_path(args.config.as_ref())?;

    match args.command {
        Command::Path(_) => {
            println!("{}", config_path.display());
        }
        Command::Show(_) => match load_config(&config_path)? {
            Some(config) => {
                println!("{}", config.to_json()?);
            }
            None => {
                println!("No config file found at {}", config_path.display());
            }
        },
        Command::Set(set_args) => {
            let mut config =
                load_config(&config_path)?.unwrap_or_else(AcbConfig::new);
            let msg = apply_set(&mut config, &set_args.setting, &set_args.values)?;
            save_config(&config, &config_path)?;
            println!("{msg}");
        }
        Command::Unset(unset_args) => {
            let mut config = load_config(&config_path)?.ok_or_else(|| {
                format!("No config file found at {}", config_path.display())
            })?;
            let msg =
                apply_unset(&mut config, &unset_args.setting, &unset_args.values)?;
            save_config(&config, &config_path)?;
            println!("{msg}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_affiliate_account() {
        let mut config = AcbConfig::new();
        let msg = apply_set(
            &mut config,
            "affiliate-account",
            &[
                "broker=questrade".into(),
                "account=12345".into(),
                "affiliate=Spouse".into(),
            ],
        )
        .unwrap();

        assert!(msg.contains("affiliate=Spouse"));
        assert_eq!(
            config.account_bindings.questrade.get("12345"),
            Some(&"Spouse".to_string())
        );
    }

    #[test]
    fn test_set_overwrites_existing() {
        let mut config = AcbConfig::new();
        config.account_bindings.etrade.insert("AAA".into(), "OldName".into());

        apply_set(
            &mut config,
            "affiliate-account",
            &[
                "broker=etrade".into(),
                "account=AAA".into(),
                "affiliate=NewName".into(),
            ],
        )
        .unwrap();

        assert_eq!(
            config.account_bindings.etrade.get("AAA"),
            Some(&"NewName".to_string())
        );
    }

    #[test]
    fn test_set_unknown_broker() {
        let mut config = AcbConfig::new();
        let err = apply_set(
            &mut config,
            "affiliate-account",
            &[
                "broker=fidelity".into(),
                "account=12345".into(),
                "affiliate=Spouse".into(),
            ],
        )
        .unwrap_err();
        assert!(err.contains("Unknown broker"));
    }

    #[test]
    fn test_set_missing_key() {
        let mut config = AcbConfig::new();
        let err = apply_set(
            &mut config,
            "affiliate-account",
            &["broker=questrade".into(), "account=12345".into()],
        )
        .unwrap_err();
        assert!(err.contains("affiliate"));
    }

    #[test]
    fn test_set_unknown_setting() {
        let mut config = AcbConfig::new();
        let err = apply_set(&mut config, "bogus", &[]).unwrap_err();
        assert!(err.contains("Unknown setting"));
    }

    #[test]
    fn test_set_bad_kv_format() {
        let mut config = AcbConfig::new();
        let err =
            apply_set(&mut config, "affiliate-account", &["not_a_pair".into()])
                .unwrap_err();
        assert!(err.contains("key=value"));
    }

    #[test]
    fn test_unset_affiliate_account() {
        let mut config = AcbConfig::new();
        config.account_bindings.rbc_di.insert("99999".into(), "Spouse".into());

        let msg = apply_unset(
            &mut config,
            "affiliate-account",
            &["broker=rbc_di".into(), "account=99999".into()],
        )
        .unwrap();

        assert!(msg.contains("Removed"));
        assert!(config.account_bindings.rbc_di.is_empty());
    }

    #[test]
    fn test_unset_not_found() {
        let mut config = AcbConfig::new();
        let err = apply_unset(
            &mut config,
            "affiliate-account",
            &["broker=questrade".into(), "account=00000".into()],
        )
        .unwrap_err();
        assert!(err.contains("No affiliate-account binding found"));
    }

    #[test]
    fn test_unset_unknown_setting() {
        let mut config = AcbConfig::new();
        let err = apply_unset(&mut config, "bogus", &[]).unwrap_err();
        assert!(err.contains("Unknown setting"));
    }
}
