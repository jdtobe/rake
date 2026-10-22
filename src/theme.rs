//! The terminal's colours and font, and how they are handed to the bundled page.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Rgb { red, green, blue }
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }

    /// AppKit reports colours as components in `0.0..=1.0`; round them to 8-bit channels.
    pub fn from_unit(red: f64, green: f64, blue: f64) -> Self {
        fn channel(component: f64) -> u8 {
            (component.clamp(0.0, 1.0) * 255.0).round() as u8
        }
        Rgb::new(channel(red), channel(green), channel(blue))
    }
}

/// Everything the page needs to look like the user's terminal. Missing parts are left to
/// xterm.js's own defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalTheme {
    pub background: Rgb,
    pub foreground: Option<Rgb>,
    pub cursor: Option<Rgb>,
    /// The 16 ANSI colours in xterm order: normal black to bright white.
    pub ansi: Option<[Rgb; 16]>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        TerminalTheme {
            background: Rgb::new(0x1e, 0x1e, 0x1e),
            foreground: None,
            cursor: None,
            ansi: None,
            font_family: Some("ui-monospace, Menlo, Monaco, monospace".to_string()),
            font_size: None,
        }
    }
}

impl TerminalTheme {
    /// A macOS font size is in points; xterm.js takes CSS pixels, and a point is 4/3 of one.
    pub fn css_font_size(point_size: f32) -> f32 {
        (point_size * 4.0 / 3.0 * 10.0).round() / 10.0
    }

    /// The configuration object embedded in the terminal page.
    pub fn config_json(&self) -> String {
        #[derive(Serialize)]
        struct Wire<'a> {
            theme: WireTheme,
            #[serde(rename = "fontFamily", skip_serializing_if = "Option::is_none")]
            font_family: Option<&'a str>,
            #[serde(rename = "fontSize", skip_serializing_if = "Option::is_none")]
            font_size: Option<f32>,
        }

        #[derive(Serialize)]
        struct WireTheme {
            background: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            foreground: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cursor: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            ansi: Option<[String; 16]>,
        }

        let wire = Wire {
            theme: WireTheme {
                background: self.background.hex(),
                foreground: self.foreground.map(Rgb::hex),
                cursor: self.cursor.map(Rgb::hex),
                ansi: self.ansi.map(|ansi| ansi.map(Rgb::hex)),
            },
            font_family: self.font_family.as_deref(),
            font_size: self.font_size,
        };
        serde_json::to_string(&wire).expect("a theme always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AFTERGLOW: [Rgb; 16] = [
        Rgb::new(0x15, 0x15, 0x15),
        Rgb::new(0xac, 0x41, 0x42),
        Rgb::new(0x7e, 0x8e, 0x50),
        Rgb::new(0xe5, 0xb5, 0x67),
        Rgb::new(0x6c, 0x99, 0xbb),
        Rgb::new(0x9f, 0x4e, 0x85),
        Rgb::new(0x7d, 0xd6, 0xcf),
        Rgb::new(0xd0, 0xd0, 0xd0),
        Rgb::new(0x50, 0x50, 0x50),
        Rgb::new(0xac, 0x41, 0x42),
        Rgb::new(0x7e, 0x8e, 0x50),
        Rgb::new(0xe5, 0xb5, 0x67),
        Rgb::new(0x6c, 0x99, 0xbb),
        Rgb::new(0x9f, 0x4e, 0x85),
        Rgb::new(0x7d, 0xd6, 0xcf),
        Rgb::new(0xf5, 0xf5, 0xf5),
    ];

    #[test]
    fn rgb_formats_as_lowercase_hex() {
        assert_eq!(Rgb::new(0x21, 0x21, 0x21).hex(), "#212121");
        assert_eq!(Rgb::new(0, 10, 255).hex(), "#000aff");
    }

    #[test]
    fn unit_components_are_rounded_and_clamped_to_eight_bits() {
        assert_eq!(Rgb::from_unit(0.0, 1.0, 0.5), Rgb::new(0, 255, 128));
        assert_eq!(Rgb::from_unit(-0.2, 1.7, 0.13), Rgb::new(0, 255, 33));
    }

    #[test]
    fn points_convert_to_rounded_css_pixels() {
        assert_eq!(TerminalTheme::css_font_size(11.0), 14.7);
        assert_eq!(TerminalTheme::css_font_size(12.0), 16.0);
    }

    #[test]
    fn default_theme_keeps_the_current_look() {
        assert_eq!(
            TerminalTheme::default().config_json(),
            r##"{"theme":{"background":"#1e1e1e"},"fontFamily":"ui-monospace, Menlo, Monaco, monospace"}"##
        );
    }

    #[test]
    fn profile_theme_carries_colours_font_and_palette() {
        let theme = TerminalTheme {
            background: Rgb::new(0x21, 0x21, 0x21),
            foreground: Some(Rgb::new(0xd0, 0xd0, 0xd0)),
            cursor: Some(Rgb::new(0xd0, 0xd0, 0xd0)),
            ansi: Some(AFTERGLOW),
            font_family: Some("Menlo".to_string()),
            font_size: Some(14.7),
        };
        assert_eq!(
            theme.config_json(),
            concat!(
                r##"{"theme":{"background":"#212121","foreground":"#d0d0d0","cursor":"#d0d0d0","##,
                r##""ansi":["#151515","#ac4142","#7e8e50","#e5b567","#6c99bb","#9f4e85","#7dd6cf","#d0d0d0","##,
                r##""#505050","#ac4142","#7e8e50","#e5b567","#6c99bb","#9f4e85","#7dd6cf","#f5f5f5"]},"##,
                r##""fontFamily":"Menlo","fontSize":14.7}"##
            )
        );
    }

    #[test]
    fn font_family_is_quoted_as_json() {
        let theme = TerminalTheme {
            font_family: Some("Weird \"Font\"".to_string()),
            ..TerminalTheme::default()
        };
        assert!(theme
            .config_json()
            .contains(r#""fontFamily":"Weird \"Font\"""#));
    }
}
