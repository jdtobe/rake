//! A shell process connected to a pseudoterminal.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

use crate::terminal_protocol::TerminalEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

pub fn default_login_shell(shell: Option<OsString>) -> ShellSpec {
    let program = shell
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"));
    ShellSpec {
        program,
        args: vec![OsString::from("-l")],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub fn new(cols: u16, rows: u16) -> Result<Self, String> {
        if cols == 0 || rows == 0 {
            return Err("terminal dimensions must be positive".to_string());
        }
        Ok(TerminalSize { cols, rows })
    }

    fn as_pty_size(self) -> PtySize {
        PtySize {
            rows: self.rows,
            cols: self.cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    running: Arc<AtomicBool>,
    _worker: JoinHandle<()>,
}

impl PtySession {
    pub fn spawn<F>(spec: ShellSpec, size: TerminalSize, notify: F) -> Result<Self, String>
    where
        F: Fn(TerminalEvent) + Send + 'static,
    {
        let pair = native_pty_system()
            .openpty(size.as_pty_size())
            .map_err(|e| format!("could not open PTY: {e}"))?;
        let mut command = CommandBuilder::new(&spec.program);
        command.args(spec.args);
        command.env("TERM", "xterm-256color");
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| format!("could not start {}: {e}", spec.program.display()))?;
        drop(pair.slave);

        let killer = child.clone_killer();
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("could not open PTY reader: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("could not open PTY writer: {e}"))?;
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker = std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buffer) {
                    Ok(0) => break,
                    Ok(count) => notify(TerminalEvent::Output(buffer[..count].to_vec())),
                    Err(error) => {
                        let _ = child.kill();
                        worker_running.store(false, Ordering::Release);
                        notify(TerminalEvent::Failed(format!(
                            "could not read PTY: {error}"
                        )));
                        return;
                    }
                }
            }
            match child.wait() {
                Ok(status) => {
                    worker_running.store(false, Ordering::Release);
                    notify(TerminalEvent::Exited {
                        code: Some(status.exit_code()),
                    });
                }
                Err(error) => {
                    worker_running.store(false, Ordering::Release);
                    notify(TerminalEvent::Failed(format!(
                        "could not wait for shell: {error}"
                    )));
                }
            }
        });

        Ok(PtySession {
            master: pair.master,
            writer,
            killer,
            running,
            _worker: worker,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        if !self.is_running() {
            return Err("terminal process has ended".to_string());
        }
        self.writer
            .write_all(bytes)
            .map_err(|e| format!("could not write to PTY: {e}"))?;
        self.writer
            .flush()
            .map_err(|e| format!("could not flush PTY input: {e}"))
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), String> {
        self.master
            .resize(size.as_pty_size())
            .map_err(|e| format!("could not resize PTY: {e}"))
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.is_running() {
            let _ = self.killer.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn default_login_shell_uses_shell_environment_value() {
        let spec = default_login_shell(Some(OsString::from("/bin/bash")));
        assert_eq!(spec.program, PathBuf::from("/bin/bash"));
        assert_eq!(spec.args, vec![OsString::from("-l")]);
    }

    #[test]
    fn default_login_shell_falls_back_to_zsh() {
        for shell in [None, Some(OsString::new())] {
            let spec = default_login_shell(shell);
            assert_eq!(spec.program, PathBuf::from("/bin/zsh"));
            assert_eq!(spec.args, vec![OsString::from("-l")]);
        }
    }

    #[test]
    fn terminal_size_rejects_zero_dimensions() {
        assert!(TerminalSize::new(0, 24).is_err());
        assert!(TerminalSize::new(80, 0).is_err());
    }

    #[test]
    fn terminal_size_accepts_positive_dimensions() {
        assert_eq!(
            TerminalSize::new(80, 24).unwrap(),
            TerminalSize { cols: 80, rows: 24 }
        );
    }

    #[test]
    fn session_delivers_all_output_before_exit() {
        let spec = ShellSpec {
            program: PathBuf::from("/bin/sh"),
            args: vec![OsString::from("-c"), OsString::from("printf rake-pty-ok")],
        };
        let (tx, rx) = mpsc::channel();
        let session = PtySession::spawn(spec, TerminalSize::new(80, 24).unwrap(), move |event| {
            tx.send(event).unwrap();
        })
        .unwrap();

        let mut output = Vec::new();
        loop {
            match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
                TerminalEvent::Output(bytes) => output.extend(bytes),
                TerminalEvent::Exited { .. } => break,
                TerminalEvent::Failed(error) => panic!("PTY failed: {error}"),
                TerminalEvent::Paste(text) => panic!("unexpected paste delivery: {text}"),
            }
        }

        assert!(
            output
                .windows(b"rake-pty-ok".len())
                .any(|window| window == b"rake-pty-ok"),
            "output was {:?}",
            String::from_utf8_lossy(&output)
        );
        assert!(!session.is_running());
    }

    #[test]
    fn completed_session_rejects_more_input() {
        let spec = ShellSpec {
            program: PathBuf::from("/bin/sh"),
            args: vec![OsString::from("-c"), OsString::from("exit 0")],
        };
        let (tx, rx) = mpsc::channel();
        let mut session =
            PtySession::spawn(spec, TerminalSize::new(80, 24).unwrap(), move |event| {
                tx.send(event).unwrap();
            })
            .unwrap();
        while !matches!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            TerminalEvent::Exited { .. }
        ) {}

        assert!(!session.is_running());
        assert!(session.write(b"echo too late\r").is_err());
    }

    #[test]
    fn dropping_running_session_does_not_wait_for_child_exit() {
        let spec = ShellSpec {
            program: PathBuf::from("/bin/cat"),
            args: Vec::new(),
        };
        let session = PtySession::spawn(spec, TerminalSize::new(80, 24).unwrap(), |_| {}).unwrap();
        drop(session);
    }
}
