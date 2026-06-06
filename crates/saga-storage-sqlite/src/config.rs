use crate::{Result, Storage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const APP_DIR: &str = "Saga";
const DB_FILE: &str = "saga.db";
const CONFIG_FILE: &str = "config.json";
const DB_PATH_ENV: &str = "SAGA_DB_PATH";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    db_path: Option<PathBuf>,
}

fn config_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_DIR).join(CONFIG_FILE))
}

fn default_db_path() -> PathBuf {
    match dirs::data_dir() {
        Some(dir) => dir.join(APP_DIR).join(DB_FILE),
        None => PathBuf::from(DB_FILE),
    }
}

fn load_config() -> Config {
    let Some(path) = config_file_path() else {
        return Config::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn choose_db_path(
    env_override: Option<String>,
    configured: Option<PathBuf>,
    default: PathBuf,
) -> PathBuf {
    match env_override {
        Some(path) => PathBuf::from(path),
        None => configured.unwrap_or(default),
    }
}

pub fn resolve_db_path() -> PathBuf {
    let env_override = std::env::var(DB_PATH_ENV).ok();
    let configured = load_config().db_path;
    choose_db_path(env_override, configured, default_db_path())
}

pub fn open_default() -> Result<Storage> {
    let path = resolve_db_path();

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    Storage::new(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_wins() {
        let chosen = choose_db_path(
            Some("/tmp/custom.db".to_string()),
            Some(PathBuf::from("/configured/saga.db")),
            PathBuf::from("/default/saga.db"),
        );
        assert_eq!(chosen, PathBuf::from("/tmp/custom.db"));
    }

    #[test]
    fn configured_used_when_no_env() {
        let chosen = choose_db_path(
            None,
            Some(PathBuf::from("/configured/saga.db")),
            PathBuf::from("/default/saga.db"),
        );
        assert_eq!(chosen, PathBuf::from("/configured/saga.db"));
    }

    #[test]
    fn default_used_when_nothing_set() {
        let chosen = choose_db_path(
            None,
            None,
            PathBuf::from("/default/saga.db"),
        );
        assert_eq!(chosen, PathBuf::from("/default/saga.db"));
    }

    #[test]
    fn default_db_path_points_at_db_file() {
        assert!(default_db_path().ends_with(DB_FILE));
    }
}
