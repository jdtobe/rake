//! Pure helpers for the menubar app, kept separate from the event loop so they can be unit tested.

pub mod animation;
pub mod config;
pub mod dropdown;
pub mod geometry;
pub mod hotkey;
pub mod terminal_protocol;

use tray_icon::menu::AboutMetadata;
use tray_icon::Icon;

pub const ICON_SIZE: u32 = 22;
pub const APP_NAME: &str = "Rake";

/// RGBA pixels for the menubar icon: a solid square with a one-pixel transparent margin.
pub fn build_icon_rgba() -> Vec<u8> {
    let edge = ICON_SIZE - 1;
    (0..ICON_SIZE * ICON_SIZE)
        .flat_map(|i| {
            let (x, y) = (i % ICON_SIZE, i / ICON_SIZE);
            let inside = x > 0 && y > 0 && x < edge && y < edge;
            [0, 0, 0, if inside { 255 } else { 0 }]
        })
        .collect()
}

pub fn build_icon() -> Icon {
    Icon::from_rgba(build_icon_rgba(), ICON_SIZE, ICON_SIZE).expect("icon buffer matches size")
}

pub fn about_metadata() -> AboutMetadata {
    AboutMetadata {
        name: Some(APP_NAME.to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_rgba_has_expected_length() {
        assert_eq!(
            build_icon_rgba().len(),
            (ICON_SIZE * ICON_SIZE * 4) as usize
        );
    }

    #[test]
    fn icon_rgba_is_not_fully_transparent() {
        assert!(build_icon_rgba().chunks(4).any(|px| px[3] != 0));
    }

    #[test]
    fn icon_is_accepted_by_tray_icon() {
        // Panics if the buffer does not match the declared dimensions.
        let _ = build_icon();
    }

    #[test]
    fn about_metadata_uses_rake_brand() {
        let meta = about_metadata();
        assert_eq!(meta.name.as_deref(), Some("Rake"));
        assert_eq!(meta.version.as_deref(), Some(env!("CARGO_PKG_VERSION")));
    }
}
