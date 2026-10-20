//! User configuration, read once at startup from `~/.config/quake-terminal/config.toml`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use global_hotkey::hotkey::HotKey;
use serde::Deserialize;

use crate::hotkey::{parse_hotkey, DEFAULT_HOTKEY};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hotkey: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hotkey: DEFAULT_HOTKEY.to_string(),
        }
    }
}

impl Config {
    /// Parse and validate a config file's contents.
    pub fn from_toml_str(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        config.hotkey()?;
        Ok(config)
    }

    pub fn hotkey(&self) -> Result<HotKey, String> {
        parse_hotkey(&self.hotkey)
    }
}

pub fn config_path(home: &Path) -> PathBuf {
    home.join(".config")
        .join("quake-terminal")
        .join("config.toml")
}

/// Load the config at `path`. A missing file means defaults; any other problem is an error
/// that names the file.
pub fn load(path: &Path) -> Result<Config, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    Config::from_toml_str(&text).map_err(|e| format!("invalid config {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, contents: Option<&str>) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("quake-terminal-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        if let Some(c) = contents {
            fs::write(&path, c).unwrap();
        }
        path
    }

    #[test]
    fn default_uses_default_hotkey() {
        assert_eq!(Config::default().hotkey, DEFAULT_HOTKEY);
        assert!(Config::default().hotkey().is_ok());
    }

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(Config::from_toml_str("").unwrap(), Config::default());
    }

    #[test]
    fn hotkey_can_be_overridden() {
        let cfg = Config::from_toml_str("hotkey = \"Alt+Space\"").unwrap();
        assert_eq!(cfg.hotkey, "Alt+Space");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = Config::from_toml_str("hotkye = \"Alt+Space\"").unwrap_err();
        assert!(err.contains("hotkye"), "error was: {err}");
    }

    #[test]
    fn malformed_toml_is_rejected() {
        assert!(Config::from_toml_str("hotkey = ").is_err());
    }

    #[test]
    fn invalid_hotkey_is_rejected_and_named() {
        let err = Config::from_toml_str("hotkey = \"Ctrl+NotAKey\"").unwrap_err();
        assert!(err.contains("Ctrl+NotAKey"), "error was: {err}");
    }

    #[test]
    fn config_path_is_under_dot_config() {
        assert_eq!(
            config_path(Path::new("/home/me")),
            PathBuf::from("/home/me/.config/quake-terminal/config.toml")
        );
    }

    #[test]
    fn load_missing_file_gives_defaults() {
        let path = temp_file("missing", None);
        assert_eq!(load(&path).unwrap(), Config::default());
    }

    #[test]
    fn load_reads_file() {
        let path = temp_file("reads", Some("hotkey = \"Alt+Space\""));
        assert_eq!(load(&path).unwrap().hotkey, "Alt+Space");
    }

    #[test]
    fn load_error_names_the_file() {
        let path = temp_file("bad", Some("hotkey = \"Ctrl+NotAKey\""));
        let err = load(&path).unwrap_err();
        assert!(err.contains(path.to_str().unwrap()), "error was: {err}");
        assert!(err.contains("Ctrl+NotAKey"), "error was: {err}");
    }
}
