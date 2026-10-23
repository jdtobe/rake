//! The system clipboard, used for copy and paste in the terminal.
//!
//! The bundled page is not a secure context (`about:blank` origin), so `navigator.clipboard` is
//! unavailable inside the webview. Copy and paste therefore go through the native pasteboard.

pub fn write(text: &str) -> Result<(), String> {
    imp::write(text)
}

pub fn read() -> Result<Option<String>, String> {
    imp::read()
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;

    pub fn write(text: &str) -> Result<(), String> {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        // SAFETY: `NSPasteboardTypeString` is a framework constant, valid for the process.
        let written = pasteboard
            .setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
        if written {
            Ok(())
        } else {
            Err("could not copy to the clipboard".to_string())
        }
    }

    pub fn read() -> Result<Option<String>, String> {
        let pasteboard = NSPasteboard::generalPasteboard();
        // SAFETY: `NSPasteboardTypeString` is a framework constant, valid for the process.
        let text = pasteboard.stringForType(unsafe { NSPasteboardTypeString });
        Ok(text.map(|text| text.to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn write(_text: &str) -> Result<(), String> {
        Err("the clipboard is not implemented on this platform".to_string())
    }

    pub fn read() -> Result<Option<String>, String> {
        Err("the clipboard is not implemented on this platform".to_string())
    }
}
