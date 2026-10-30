//! A shell process connected to a pseudoterminal.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

use crate::process::{self, JobState, ProcessIdentity, SessionObservation};
use crate::terminal_protocol::TerminalEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: Option<PathBuf>,
}

pub fn default_login_shell(shell: Option<OsString>) -> ShellSpec {
    let program = shell
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"));
    ShellSpec {
        program,
        args: vec![OsString::from("-l")],
        cwd: None,
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
    identity: Option<ProcessIdentity>,
    cleanup: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    cleanup_result: Option<Result<(), String>>,
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
        if let Some(cwd) = spec.cwd {
            command.cwd(cwd);
        }
        command.env("TERM", "xterm-256color");
        // Acquire fallible I/O handles before starting a child we would need to reap.
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("could not open PTY reader: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("could not open PTY writer: {e}"))?;
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| format!("could not start {}: {e}", spec.program.display()))?;
        drop(pair.slave);

        let identity = child.process_id().and_then(process::identity);

        let killer = child.clone_killer();
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = Arc::clone(&running);
        let worker = std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buffer) {
                    Ok(0) => break,
                    Ok(count) => notify(TerminalEvent::Output(buffer[..count].to_vec())),
                    Err(error) => {
                        #[cfg(target_os = "macos")]
                        if let Some(id) = identity {
                            if let Err(cleanup_error) =
                                process::terminate(id, process::snapshot(id), child.clone_killer())
                            {
                                notify(TerminalEvent::Failed(format!(
                                    "PTY cleanup: {cleanup_error}"
                                )));
                            }
                        }
                        if matches!(child.try_wait(), Ok(Some(_))) {
                            worker_running.store(false, Ordering::Release);
                        }
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
            identity,
            cleanup: None,
            cleanup_result: None,
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

    pub fn observe(&self) -> SessionObservation {
        if !self.is_running() {
            return SessionObservation {
                cwd: None,
                jobs: JobState::Ended,
            };
        }
        #[cfg(target_os = "macos")]
        if let Some(identity) = self.identity {
            return process::observe(identity, self.master.as_raw_fd());
        }
        SessionObservation {
            cwd: None,
            jobs: JobState::Unknown,
        }
    }

    fn start_cleanup(&mut self) {
        if self.cleanup.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.cleanup = Some(rx);
        let identity = self.identity;
        let killer = self.killer.clone_killer();
        #[cfg(target_os = "macos")]
        let captured = identity.map(process::snapshot);
        std::thread::spawn(move || {
            #[cfg(target_os = "macos")]
            let result = match (identity, captured) {
                (Some(id), Some(targets)) => process::terminate(id, targets, killer),
                _ => Err("shell identity unavailable; refusing unverified cleanup".into()),
            };
            #[cfg(not(target_os = "macos"))]
            let result: Result<(), String> = {
                let _ = (identity, killer);
                Err("process cleanup unsupported on this platform".into())
            };
            if let Err(error) = &result {
                eprintln!("PTY cleanup: {error}");
            }
            let _ = tx.send(result);
        });
    }

    /// Bounded cleanup of the captured local session tree, including stopped jobs.
    /// Detached/reparented daemons outside that tree cannot be reclaimed safely.
    pub fn terminate(&mut self) -> Result<(), String> {
        if let Some(result) = &self.cleanup_result {
            return result.clone();
        }
        self.start_cleanup();
        let result = self
            .cleanup
            .as_ref()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(3))
            .map_err(|error| format!("could not confirm PTY cleanup: {error}"))?;
        self.cleanup_result = Some(result.clone());
        result
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.cleanup.is_none() {
            self.start_cleanup();
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
            cwd: None,
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
            cwd: None,
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
            cwd: None,
        };
        let session = PtySession::spawn(spec, TerminalSize::new(80, 24).unwrap(), |_| {}).unwrap();
        drop(session);
    }

    #[cfg(target_os = "macos")]
    struct TestSession(Option<PtySession>);

    #[cfg(target_os = "macos")]
    impl std::ops::Deref for TestSession {
        type Target = PtySession;
        fn deref(&self) -> &Self::Target {
            self.0.as_ref().unwrap()
        }
    }

    #[cfg(target_os = "macos")]
    impl std::ops::DerefMut for TestSession {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0.as_mut().unwrap()
        }
    }

    #[cfg(target_os = "macos")]
    impl Drop for TestSession {
        fn drop(&mut self) {
            if let Some(session) = &mut self.0 {
                // Also runs during assertion unwinding; never leave native jobs
                // behind just because their test failed before explicit cleanup.
                if let Err(error) = session.terminate() {
                    eprintln!("test cleanup: {error}");
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn shell(cwd: Option<PathBuf>) -> (TestSession, mpsc::Receiver<TerminalEvent>) {
        let (tx, rx) = mpsc::channel();
        let session = PtySession::spawn(
            ShellSpec {
                program: "/bin/sh".into(),
                args: vec!["-i".into()],
                cwd,
            },
            TerminalSize::new(80, 24).unwrap(),
            move |event| {
                let _ = tx.send(event);
            },
        )
        .unwrap();
        (TestSession(Some(session)), rx)
    }

    #[cfg(target_os = "macos")]
    fn marker(session: &mut PtySession, rx: &mpsc::Receiver<TerminalEvent>, command: &str) {
        session
            .write(
                format!("{command}\nprintf '\\162\\145\\141\\144\\171\\055\\117\\113\\n'\n")
                    .as_bytes(),
            )
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut output = Vec::new();
        loop {
            match rx
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .unwrap()
            {
                TerminalEvent::Output(bytes) => output.extend(bytes),
                event => panic!("unexpected {event:?}"),
            }
            if output.windows(8).any(|s| s == b"ready-OK") {
                break;
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn await_jobs(session: &PtySession, expected: crate::process::JobState) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let observation = session.observe();
            if observation.jobs == expected {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "last observation: {observation:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn shell_starts_in_requested_cwd() {
        let cwd = std::env::temp_dir().canonicalize().unwrap();
        let (mut session, rx) = shell(Some(cwd.clone()));
        marker(&mut session, &rx, ":");
        assert_eq!(session.observe().cwd, Some(cwd));
        marker(&mut session, &rx, "cd /");
        assert_eq!(session.observe().cwd, Some("/".into()));
        session.write(b"/bin/sleep 60\n").unwrap();
        await_jobs(&session, crate::process::JobState::Busy);
        assert_eq!(session.observe().cwd, Some("/".into()));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn idle_shell_is_not_a_running_app() {
        let (mut session, rx) = shell(None);
        marker(&mut session, &rx, ":");
        await_jobs(&session, crate::process::JobState::Idle);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn foreground_pipeline_is_busy() {
        let (mut session, rx) = shell(None);
        marker(&mut session, &rx, ":");
        session.write(b"/bin/sleep 60 | /bin/cat\n").unwrap();
        await_jobs(&session, crate::process::JobState::Busy);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn stopped_job_is_busy() {
        let (mut session, rx) = shell(None);
        marker(
            &mut session,
            &rx,
            "/bin/sh -c 'kill -STOP $$; exec /bin/sleep 60' &",
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let targets = loop {
            let targets = crate::process::snapshot(session.identity.unwrap()).unwrap();
            if targets.iter().any(|id| crate::process::is_stopped(*id)) {
                break targets;
            }
            assert!(std::time::Instant::now() < deadline, "job did not stop");
            std::thread::sleep(Duration::from_millis(10));
        };
        await_jobs(&session, crate::process::JobState::Busy);
        session.terminate().unwrap();
        assert!(targets
            .into_iter()
            .all(|id| !crate::process::is_live(id).unwrap()));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn background_job_is_busy() {
        let (mut session, rx) = shell(None);
        marker(&mut session, &rx, "/bin/sleep 60 &");
        await_jobs(&session, crate::process::JobState::Busy);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn ended_session_is_ended() {
        let (mut session, rx) = shell(None);
        session.write(b"exit\n").unwrap();
        while !matches!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            TerminalEvent::Exited { .. }
        ) {}
        assert_eq!(session.observe().jobs, crate::process::JobState::Ended);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn terminate_stops_shell_foreground_and_background_jobs() {
        let (mut session, rx) = shell(None);
        // Ignored dispositions survive exec: shell and jobs require escalation,
        // rather than letting a successful HUP masquerade as complete cleanup.
        marker(&mut session, &rx, "trap '' HUP TERM; /bin/sleep 60 &");
        session.write(b"/bin/sleep 60 | /bin/cat\n").unwrap();
        await_jobs(&session, crate::process::JobState::Busy);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let targets = loop {
            let targets = crate::process::snapshot(session.identity.unwrap()).unwrap();
            if targets.len() >= 4 {
                break targets;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        };
        session.terminate().unwrap();
        assert!(targets
            .into_iter()
            .all(|id| !crate::process::is_live(id).unwrap()));
        session.terminate().unwrap(); // completed cleanup is idempotent
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn dropping_idle_session_does_not_wait() {
        let (mut session, rx) = shell(None);
        marker(&mut session, &rx, ":");
        let identity = session.identity.unwrap();
        let native = session.0.take().unwrap();
        let start = std::time::Instant::now();
        drop(native);
        assert!(start.elapsed() < Duration::from_millis(200));
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while crate::process::is_live(identity).unwrap() {
            assert!(
                std::time::Instant::now() < deadline,
                "dropped shell survived cleanup"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
