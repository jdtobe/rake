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
    /// Window height as a percentage of the monitor's height.
    pub height_percent: u8,
    /// Window width as a percentage of the monitor's width. The window is centered.
    pub width_percent: u8,
    /// Hide the window when it loses keyboard focus.
    pub hide_on_focus_lost: bool,
    /// How long the slide in/out takes, in milliseconds. 0 turns the animation off.
    pub animation_ms: u32,
}

pub const ANIMATION_MS_MAX: u32 = 2000;

pub const HEIGHT_PERCENT_RANGE: std::ops::RangeInclusive<u8> = 10..=100;
pub const WIDTH_PERCENT_RANGE: std::ops::RangeInclusive<u8> = 10..=100;

impl Default for Config {
    fn default() -> Self {
        Config {
            hotkey: DEFAULT_HOTKEY.to_string(),
            height_percent: 40,
            width_percent: 100,
            hide_on_focus_lost: false,
            animation_ms: 300,
        }
    }
}

impl Config {
    /// Parse and validate a config file's contents.
    pub fn from_toml_str(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        config.hotkey()?;
        for (name, value, range) in [
            (
                "height_percent",
                config.height_percent,
                HEIGHT_PERCENT_RANGE,
            ),
            ("width_percent", config.width_percent, WIDTH_PERCENT_RANGE),
        ] {
            if !range.contains(&value) {
                return Err(format!(
                    "{name} must be between {} and {}, got {value}",
                    range.start(),
                    range.end()
                ));
            }
        }
        if config.animation_ms > ANIMATION_MS_MAX {
            return Err(format!(
                "animation_ms must be between 0 and {ANIMATION_MS_MAX}, got {}",
                config.animation_ms
            ));
        }
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
    fn height_defaults_to_40_percent_and_can_be_overridden() {
        assert_eq!(Config::default().height_percent, 40);
        let cfg = Config::from_toml_str("height_percent = 60").unwrap();
        assert_eq!(cfg.height_percent, 60);
    }

    #[test]
    fn height_outside_10_to_100_is_rejected() {
        for bad in ["9", "101", "300", "-5", "0"] {
            let err = Config::from_toml_str(&format!("height_percent = {bad}")).unwrap_err();
            assert!(err.contains("height_percent"), "{bad}: error was: {err}");
        }
        assert!(Config::from_toml_str("height_percent = 10").is_ok());
        assert!(Config::from_toml_str("height_percent = 100").is_ok());
    }

    #[test]
    fn width_defaults_to_full_and_can_be_overridden() {
        assert_eq!(Config::default().width_percent, 100);
        let cfg = Config::from_toml_str("width_percent = 70").unwrap();
        assert_eq!(cfg.width_percent, 70);
    }

    #[test]
    fn width_outside_10_to_100_is_rejected() {
        for bad in ["9", "101", "300", "-5", "0"] {
            let err = Config::from_toml_str(&format!("width_percent = {bad}")).unwrap_err();
            assert!(err.contains("width_percent"), "{bad}: error was: {err}");
        }
        assert!(Config::from_toml_str("width_percent = 10").is_ok());
        assert!(Config::from_toml_str("width_percent = 100").is_ok());
    }

    #[test]
    fn hide_on_focus_lost_defaults_off_and_can_be_enabled() {
        assert!(!Config::default().hide_on_focus_lost);
        let cfg = Config::from_toml_str("hide_on_focus_lost = true").unwrap();
        assert!(cfg.hide_on_focus_lost);
    }

    #[test]
    fn hide_on_focus_lost_must_be_a_boolean() {
        let err = Config::from_toml_str("hide_on_focus_lost = \"yes\"").unwrap_err();
        assert!(err.contains("hide_on_focus_lost"), "error was: {err}");
    }

    #[test]
    fn animation_defaults_to_300ms_and_can_be_overridden_or_disabled() {
        assert_eq!(Config::default().animation_ms, 300);
        assert_eq!(
            Config::from_toml_str("animation_ms = 350")
                .unwrap()
                .animation_ms,
            350
        );
        assert_eq!(
            Config::from_toml_str("animation_ms = 0")
                .unwrap()
                .animation_ms,
            0
        );
    }

    #[test]
    fn animation_longer_than_2_seconds_or_negative_is_rejected() {
        assert!(Config::from_toml_str("animation_ms = 2000").is_ok());
        for bad in ["2001", "-1"] {
            let err = Config::from_toml_str(&format!("animation_ms = {bad}")).unwrap_err();
            assert!(err.contains("animation_ms"), "{bad}: error was: {err}");
        }
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
