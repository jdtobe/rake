//! Global hotkey parsing and the show/hide toggle state.

use global_hotkey::hotkey::HotKey;
use global_hotkey::HotKeyState;

pub const DEFAULT_HOTKEY: &str = "Ctrl+`";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Hidden,
    Shown,
}

impl Visibility {
    pub fn toggled(self) -> Self {
        match self {
            Visibility::Hidden => Visibility::Shown,
            Visibility::Shown => Visibility::Hidden,
        }
    }

    /// What to do when the window gains or loses focus.
    pub fn after_focus_change(self, hide_on_focus_lost: bool, focused: bool) -> Self {
        if hide_on_focus_lost && !focused {
            Visibility::Hidden
        } else {
            self
        }
    }
}

pub fn parse_hotkey(text: &str) -> Result<HotKey, String> {
    text.parse()
        .map_err(|e| format!("invalid hotkey {text:?}: {e}"))
}

/// Only a key press toggles; the matching release must be ignored.
pub fn should_toggle(state: HotKeyState) -> bool {
    state == HotKeyState::Pressed
}

#[cfg(test)]
mod tests {
    use super::*;
    use global_hotkey::hotkey::{Code, Modifiers};

    #[test]
    fn default_hotkey_parses_to_ctrl_backquote() {
        let key = parse_hotkey(DEFAULT_HOTKEY).unwrap();
        assert_eq!(key, HotKey::new(Some(Modifiers::CONTROL), Code::Backquote));
    }

    #[test]
    fn parses_other_combinations() {
        let key = parse_hotkey("Alt+Space").unwrap();
        assert_eq!(key, HotKey::new(Some(Modifiers::ALT), Code::Space));
    }

    #[test]
    fn rejects_garbage_and_names_the_input() {
        let err = parse_hotkey("Ctrl+NotAKey").unwrap_err();
        assert!(err.contains("Ctrl+NotAKey"), "error was: {err}");
    }

    #[test]
    fn rejects_modifier_only_input() {
        assert!(parse_hotkey("Ctrl").is_err());
    }

    #[test]
    fn visibility_toggles_both_ways() {
        assert_eq!(Visibility::Hidden.toggled(), Visibility::Shown);
        assert_eq!(Visibility::Shown.toggled(), Visibility::Hidden);
    }

    #[test]
    fn losing_focus_hides_only_when_enabled() {
        assert_eq!(
            Visibility::Shown.after_focus_change(true, false),
            Visibility::Hidden
        );
        assert_eq!(
            Visibility::Shown.after_focus_change(false, false),
            Visibility::Shown
        );
    }

    #[test]
    fn gaining_focus_never_changes_visibility() {
        for hide in [true, false] {
            assert_eq!(
                Visibility::Shown.after_focus_change(hide, true),
                Visibility::Shown
            );
            assert_eq!(
                Visibility::Hidden.after_focus_change(hide, true),
                Visibility::Hidden
            );
        }
    }

    #[test]
    fn losing_focus_while_hidden_stays_hidden() {
        assert_eq!(
            Visibility::Hidden.after_focus_change(true, false),
            Visibility::Hidden
        );
    }

    #[test]
    fn only_press_toggles() {
        assert!(should_toggle(HotKeyState::Pressed));
        assert!(!should_toggle(HotKeyState::Released));
    }
}
