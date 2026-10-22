//! Messages crossing the Rust, PTY and terminal-webview boundaries.

use std::collections::VecDeque;

use base64::Engine;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalRequest {
    Ready,
    Input(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RequestKind {
    Ready,
    Input,
    Resize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    #[serde(rename = "type")]
    kind: RequestKind,
    data: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
}

pub fn parse_request(body: &str) -> Result<TerminalRequest, String> {
    let wire: WireRequest =
        serde_json::from_str(body).map_err(|e| format!("invalid terminal request: {e}"))?;
    match (wire.kind, wire.data, wire.cols, wire.rows) {
        (RequestKind::Ready, None, None, None) => Ok(TerminalRequest::Ready),
        (RequestKind::Input, Some(data), None, None) => {
            Ok(TerminalRequest::Input(data.into_bytes()))
        }
        (RequestKind::Resize, None, Some(0), Some(_)) => {
            Err("terminal columns must be positive".to_string())
        }
        (RequestKind::Resize, None, Some(_), Some(0)) => {
            Err("terminal rows must be positive".to_string())
        }
        (RequestKind::Resize, None, Some(cols), Some(rows)) => {
            Ok(TerminalRequest::Resize { cols, rows })
        }
        _ => Err("terminal request has fields that do not match its type".to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    Output(Vec<u8>),
    Exited { code: Option<u32> },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalDelivery {
    Output(Vec<u8>),
    Exited { code: Option<u32> },
    Failed(String),
}

#[derive(Default)]
pub struct TerminalBridge {
    ready: bool,
    pending: VecDeque<TerminalDelivery>,
}

impl TerminalBridge {
    pub fn push(&mut self, event: TerminalEvent) {
        let delivery = match event {
            TerminalEvent::Output(bytes) => {
                if let Some(TerminalDelivery::Output(previous)) = self.pending.back_mut() {
                    previous.extend(bytes);
                    return;
                }
                TerminalDelivery::Output(bytes)
            }
            TerminalEvent::Exited { code } => TerminalDelivery::Exited { code },
            TerminalEvent::Failed(message) => TerminalDelivery::Failed(message),
        };
        self.pending.push_back(delivery);
    }

    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    pub fn take_deliveries(&mut self) -> Vec<TerminalDelivery> {
        if !self.ready {
            return Vec::new();
        }
        self.pending.drain(..).collect()
    }

    pub fn restore_front(&mut self, deliveries: Vec<TerminalDelivery>) {
        for delivery in deliveries.into_iter().rev() {
            self.pending.push_front(delivery);
        }
    }
}

pub fn delivery_script(delivery: &TerminalDelivery) -> String {
    match delivery {
        TerminalDelivery::Output(bytes) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!(
                "window.rake.writeBase64({});",
                serde_json::to_string(&encoded).expect("a string always serializes")
            )
        }
        TerminalDelivery::Exited { code } => {
            let message = match code {
                Some(code) => format!("Process ended with status {code}."),
                None => "Process ended.".to_string(),
            };
            format!(
                "window.rake.end({});",
                serde_json::to_string(&message).expect("a string always serializes")
            )
        }
        TerminalDelivery::Failed(error) => {
            let message = format!("Terminal error: {error}");
            format!(
                "window.rake.end({});",
                serde_json::to_string(&message).expect("a string always serializes")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ready_request() {
        assert_eq!(
            parse_request(r#"{"type":"ready"}"#).unwrap(),
            TerminalRequest::Ready
        );
    }

    #[test]
    fn parse_input_request_as_utf8_bytes() {
        assert_eq!(
            parse_request(r#"{"type":"input","data":"printf 'hi'\r"}"#).unwrap(),
            TerminalRequest::Input(b"printf 'hi'\r".to_vec())
        );
    }

    #[test]
    fn parse_resize_request() {
        assert_eq!(
            parse_request(r#"{"type":"resize","cols":120,"rows":40}"#).unwrap(),
            TerminalRequest::Resize {
                cols: 120,
                rows: 40
            }
        );
    }

    #[test]
    fn parse_rejects_malformed_unknown_and_extra_fields() {
        for body in [
            "not json",
            r#"{"type":"wat"}"#,
            r#"{"type":"ready","extra":true}"#,
        ] {
            assert!(parse_request(body).is_err(), "accepted {body}");
        }
    }

    #[test]
    fn parse_rejects_zero_or_oversized_dimensions() {
        for body in [
            r#"{"type":"resize","cols":0,"rows":24}"#,
            r#"{"type":"resize","cols":80,"rows":0}"#,
            r#"{"type":"resize","cols":65536,"rows":24}"#,
        ] {
            assert!(parse_request(body).is_err(), "accepted {body}");
        }
    }

    #[test]
    fn bridge_holds_output_until_ready() {
        let mut bridge = TerminalBridge::default();
        bridge.push(TerminalEvent::Output(b"early".to_vec()));
        assert!(bridge.take_deliveries().is_empty());

        bridge.mark_ready();
        assert_eq!(
            bridge.take_deliveries(),
            vec![TerminalDelivery::Output(b"early".to_vec())]
        );
    }

    #[test]
    fn bridge_coalesces_adjacent_output_without_changing_bytes() {
        let mut bridge = TerminalBridge::default();
        bridge.mark_ready();
        bridge.push(TerminalEvent::Output(vec![0, 0xff]));
        bridge.push(TerminalEvent::Output(vec![0x1b, b'[', b'm']));
        assert_eq!(
            bridge.take_deliveries(),
            vec![TerminalDelivery::Output(vec![0, 0xff, 0x1b, b'[', b'm'])]
        );
    }

    #[test]
    fn bridge_delivers_output_before_exit_or_failure() {
        for ending in [
            TerminalEvent::Exited { code: Some(7) },
            TerminalEvent::Failed("read failed".to_string()),
        ] {
            let mut bridge = TerminalBridge::default();
            bridge.mark_ready();
            bridge.push(TerminalEvent::Output(b"last output".to_vec()));
            bridge.push(ending.clone());
            let deliveries = bridge.take_deliveries();
            assert!(matches!(deliveries[0], TerminalDelivery::Output(_)));
            assert!(!matches!(deliveries[1], TerminalDelivery::Output(_)));
        }
    }

    #[test]
    fn bridge_restores_undelivered_items_at_the_front() {
        let mut bridge = TerminalBridge::default();
        bridge.mark_ready();
        bridge.push(TerminalEvent::Output(b"later".to_vec()));
        let later = bridge.take_deliveries();
        bridge.push(TerminalEvent::Exited { code: None });
        bridge.restore_front(later);
        assert_eq!(
            bridge.take_deliveries(),
            vec![
                TerminalDelivery::Output(b"later".to_vec()),
                TerminalDelivery::Exited { code: None }
            ]
        );
    }

    #[test]
    fn delivery_script_is_binary_safe_and_quotes_errors() {
        assert_eq!(
            delivery_script(&TerminalDelivery::Output(vec![0, 0xff])),
            r#"window.rake.writeBase64("AP8=");"#
        );
        assert_eq!(
            delivery_script(&TerminalDelivery::Failed("bad \"quote\"\nnext".to_string())),
            "window.rake.end(\"Terminal error: bad \\\"quote\\\"\\nnext\");"
        );
    }
}
