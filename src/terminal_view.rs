//! The local xterm.js document and its native webview.

use crate::terminal_protocol::{delivery_script, TerminalBridge, TerminalDelivery, TerminalEvent};
use crate::theme::TerminalTheme;
use tao::dpi::{LogicalPosition, PhysicalSize};
use tao::window::Window;
use wry::{NewWindowResponse, Rect, WebView, WebViewBuilder};

const INDEX_HTML: &str = include_str!("../assets/terminal/index.html");
const XTERM_CSS: &str = include_str!("../assets/terminal/xterm.css");
const XTERM_JS: &str = include_str!("../assets/terminal/xterm.js");
const ADDON_FIT_JS: &str = include_str!("../assets/terminal/addon-fit.js");
const RAKE_JS: &str = include_str!("../assets/terminal/rake.js");

pub fn terminal_html(theme: &TerminalTheme) -> String {
    let rake_js = RAKE_JS.replace("/*__RAKE_CONFIG__*/", &theme.config_json());
    INDEX_HTML
        .replace("/*__THEME_BACKGROUND__*/", &theme.background.hex())
        .replace("/*__XTERM_CSS__*/", XTERM_CSS)
        .replace("/*__XTERM_JS__*/", XTERM_JS)
        .replace("/*__ADDON_FIT_JS__*/", ADDON_FIT_JS)
        .replace("/*__RAKE_JS__*/", &rake_js)
}

fn navigation_allowed(url: &str) -> bool {
    url == "about:blank" || url.starts_with("data:text/html")
}

struct ScriptDelivery {
    delivery: TerminalDelivery,
    script: String,
}

#[derive(Default)]
struct DeliveryQueue {
    bridge: TerminalBridge,
}

impl DeliveryQueue {
    fn push(&mut self, event: TerminalEvent) {
        self.bridge.push(event);
    }

    fn mark_ready(&mut self) {
        self.bridge.mark_ready();
    }

    fn take_scripts(&mut self) -> Vec<ScriptDelivery> {
        self.bridge
            .take_deliveries()
            .into_iter()
            .map(|delivery| ScriptDelivery {
                script: delivery_script(&delivery),
                delivery,
            })
            .collect()
    }

    fn restore(&mut self, deliveries: Vec<TerminalDelivery>) {
        self.bridge.restore_front(deliveries);
    }
}

pub struct TerminalView {
    webview: WebView,
    deliveries: DeliveryQueue,
}

impl TerminalView {
    pub fn new<F>(window: &Window, theme: &TerminalTheme, ipc: F) -> Result<Self, String>
    where
        F: Fn(String) + 'static,
    {
        let webview = WebViewBuilder::new()
            .with_html(terminal_html(theme))
            .with_ipc_handler(move |request| ipc(request.body().clone()))
            .with_navigation_handler(|url| navigation_allowed(&url))
            .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
            .build(window)
            .map_err(|e| format!("could not create terminal webview: {e}"))?;
        Ok(TerminalView {
            webview,
            deliveries: DeliveryQueue::default(),
        })
    }

    pub fn mark_ready(&mut self) {
        self.deliveries.mark_ready();
    }

    pub fn queue(&mut self, event: TerminalEvent) {
        self.deliveries.push(event);
    }

    pub fn flush(&mut self) -> Result<(), String> {
        let mut prepared = self.deliveries.take_scripts().into_iter();
        while let Some(item) = prepared.next() {
            if let Err(error) = self.webview.evaluate_script(&item.script) {
                let mut undelivered = vec![item.delivery];
                undelivered.extend(prepared.map(|remaining| remaining.delivery));
                self.deliveries.restore(undelivered);
                return Err(format!("could not update terminal webview: {error}"));
            }
        }
        Ok(())
    }

    pub fn focus(&self) -> Result<(), String> {
        self.webview
            .evaluate_script("window.rake?.focus();")
            .map_err(|e| format!("could not focus terminal webview: {e}"))
    }

    pub fn set_bounds(&self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<(), String> {
        let logical = size.to_logical::<u32>(scale_factor);
        self.webview
            .set_bounds(Rect {
                position: LogicalPosition::new(0, 0).into(),
                size: logical.into(),
            })
            .map_err(|e| format!("could not resize terminal webview: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_protocol::TerminalEvent;
    use crate::theme::{Rgb, TerminalTheme};

    #[test]
    fn terminal_document_bundles_pinned_assets_without_remote_loads() {
        let html = terminal_html(&TerminalTheme::default());
        assert!(html.contains("xterm.js 6.0.0"));
        assert!(html.contains("addon-fit 0.11.0"));
        assert!(!html.contains("<script src=\"http"));
        assert!(!html.contains("<link href=\"http"));
    }

    #[test]
    fn terminal_document_applies_the_given_theme() {
        let theme = TerminalTheme {
            background: Rgb::new(0x21, 0x21, 0x21),
            foreground: Some(Rgb::new(0xd0, 0xd0, 0xd0)),
            cursor: None,
            ansi: None,
            font_family: Some("Menlo".to_string()),
            font_size: Some(14.7),
        };
        let html = terminal_html(&theme);
        assert!(html.contains(r##""foreground":"#d0d0d0""##));
        assert!(html.contains(r##""fontFamily":"Menlo""##));
        assert!(html.contains(r##""fontSize":14.7"##));
        assert!(html.contains("background: #212121"));
        assert!(
            !html.contains("__RAKE_CONFIG__"),
            "theme placeholder left behind"
        );
        assert!(
            !html.contains("__THEME_BACKGROUND__"),
            "background placeholder left behind"
        );
    }

    #[test]
    fn terminal_document_fills_the_window_and_installs_rake_bridge() {
        let html = terminal_html(&TerminalTheme::default());
        assert!(html.contains("id=\"terminal\""));
        assert!(html.contains("height: 100%"));
        assert!(html.contains("window.rake"));
        for operation in ["writeBase64", "end", "focus"] {
            assert!(html.contains(operation), "missing {operation}");
        }
    }

    #[test]
    fn terminal_document_reports_readiness_input_and_positive_size() {
        let html = terminal_html(&TerminalTheme::default());
        for message_type in ["ready", "input", "resize"] {
            assert!(
                html.contains(&format!("type: '{message_type}'")),
                "missing {message_type} message"
            );
        }
        assert!(html.contains("cols > 0 && rows > 0"));
        assert!(html.contains("terminal.onData"));
        assert!(html.contains("new ResizeObserver"));
    }

    #[test]
    fn delivery_queue_waits_for_ready_and_drains_once() {
        let mut queue = DeliveryQueue::default();
        queue.push(TerminalEvent::Output(b"one".to_vec()));
        assert!(queue.take_scripts().is_empty());

        queue.mark_ready();
        assert_eq!(
            queue
                .take_scripts()
                .into_iter()
                .map(|delivery| delivery.script)
                .collect::<Vec<_>>(),
            vec![r#"window.rake.writeBase64("b25l");"#.to_string()]
        );
        assert!(queue.take_scripts().is_empty());
    }

    #[test]
    fn delivery_queue_coalesces_output_and_preserves_exit_order() {
        let mut queue = DeliveryQueue::default();
        queue.mark_ready();
        queue.push(TerminalEvent::Output(b"a".to_vec()));
        queue.push(TerminalEvent::Output(b"b".to_vec()));
        queue.push(TerminalEvent::Exited { code: Some(0) });
        let scripts: Vec<String> = queue
            .take_scripts()
            .into_iter()
            .map(|delivery| delivery.script)
            .collect();
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0], r#"window.rake.writeBase64("YWI=");"#);
        assert_eq!(
            scripts[1],
            r#"window.rake.end("Process ended with status 0.");"#
        );
    }

    #[test]
    fn terminal_document_uses_the_native_clipboard() {
        let html = terminal_html(&TerminalTheme::default());
        for request in ["copy", "paste"] {
            assert!(
                html.contains(&format!("type: '{request}'")),
                "missing {request} request"
            );
        }
        assert!(html.contains("paste(text)"), "missing bridge paste");
        assert!(
            !html.contains("navigator.clipboard"),
            "the page is not a secure context, so it has no clipboard API"
        );
    }

    #[test]
    fn navigation_allows_embedded_document_but_denies_external_urls() {
        assert!(navigation_allowed("about:blank"));
        assert!(navigation_allowed("data:text/html;charset=utf-8,terminal"));
        for url in [
            "https://example.com",
            "http://example.com",
            "file:///tmp/page.html",
        ] {
            assert!(!navigation_allowed(url), "allowed {url}");
        }
    }
}
