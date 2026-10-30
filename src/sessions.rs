//! Own PTY lifetimes and isolate readers from the UI's bounded output bridge.
use crate::process::{JobState, SessionObservation};
use crate::pty::{PtySession, ShellSpec, TerminalSize};
use crate::session_output::SessionOutput;
use crate::tabs::TabId;
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
pub struct SessionUpdates {
    pub observations: BTreeMap<TabId, SessionObservation>,
    pub errors: Vec<String>,
}

struct Inner {
    sessions: Mutex<BTreeMap<TabId, Arc<Mutex<PtySession>>>>,
    updates: Mutex<SessionUpdates>,
    stopped: Mutex<bool>,
    stop: Condvar,
    output: SessionOutput,
}

pub struct Sessions(Arc<Inner>);

impl Sessions {
    pub fn new(output: SessionOutput) -> Self {
        let inner = Arc::new(Inner {
            sessions: Mutex::new(BTreeMap::new()),
            updates: Mutex::new(SessionUpdates::default()),
            stopped: Mutex::new(false),
            stop: Condvar::new(),
            output,
        });
        let weak = Arc::downgrade(&inner);
        std::thread::spawn(move || {
            let mut previous = BTreeMap::new();
            while let Some(inner) = weak.upgrade() {
                let stopped = inner.stopped.lock().unwrap();
                let (stopped, _) = inner
                    .stop
                    .wait_timeout(stopped, std::time::Duration::from_secs(2))
                    .unwrap();
                if *stopped {
                    break;
                }
                drop(stopped);
                let handles: Vec<_> = inner
                    .sessions
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(id, session)| (*id, session.clone()))
                    .collect();
                let mut changed = false;
                for (id, session) in handles {
                    let observation = session.lock().unwrap().observe();
                    let live = inner.sessions.lock().unwrap();
                    if live.contains_key(&id) && previous.get(&id) != Some(&observation) {
                        inner
                            .updates
                            .lock()
                            .unwrap()
                            .observations
                            .insert(id, observation.clone());
                        previous.insert(id, observation);
                        changed = true;
                    }
                }
                previous.retain(|id, _| inner.sessions.lock().unwrap().contains_key(id));
                if changed {
                    inner.output.wake();
                }
            }
        });
        Self(inner)
    }

    pub fn spawn(&mut self, id: TabId, spec: ShellSpec, size: TerminalSize) -> Result<(), String> {
        if self.contains(id) {
            return Err("duplicate session ID".into());
        }
        self.0.output.register(id);
        let output = self.0.output.clone();
        let session = match PtySession::spawn(spec, size, move |event| {
            let _ = output.send(id, event);
        }) {
            Ok(session) => session,
            Err(error) => {
                self.0.output.close(id);
                return Err(error);
            }
        };
        self.0
            .sessions
            .lock()
            .unwrap()
            .insert(id, Arc::new(Mutex::new(session)));
        Ok(())
    }

    fn get(&self, id: TabId) -> Option<Arc<Mutex<PtySession>>> {
        self.0.sessions.lock().unwrap().get(&id).cloned()
    }
    pub fn contains(&self, id: TabId) -> bool {
        self.get(id).is_some()
    }
    pub fn write(&mut self, id: TabId, bytes: &[u8]) -> Result<(), String> {
        self.get(id)
            .map_or(Ok(()), |session| session.lock().unwrap().write(bytes))
    }
    pub fn resize(&mut self, id: TabId, size: TerminalSize) -> Result<(), String> {
        self.get(id)
            .map_or(Ok(()), |session| session.lock().unwrap().resize(size))
    }
    pub fn observe(&self, id: TabId) -> SessionObservation {
        self.get(id).map_or(
            SessionObservation {
                cwd: None,
                jobs: JobState::Ended,
            },
            |session| session.lock().unwrap().observe(),
        )
    }
    pub fn take_updates(&self) -> SessionUpdates {
        std::mem::take(&mut *self.0.updates.lock().unwrap())
    }

    pub fn remove(&mut self, id: TabId) -> Result<(), String> {
        self.0.output.close(id);
        let session = self.0.sessions.lock().unwrap().remove(&id);
        self.0.updates.lock().unwrap().observations.remove(&id);
        if let Some(session) = session {
            let weak = Arc::downgrade(&self.0);
            std::thread::spawn(move || {
                if let Err(error) = session.lock().unwrap().terminate() {
                    if let Some(inner) = weak.upgrade() {
                        let mut updates = inner.updates.lock().unwrap();
                        if updates.errors.len() < 16 {
                            updates.errors.push(error);
                        }
                        drop(updates);
                        inner.output.wake();
                    }
                }
            });
        }
        Ok(())
    }
}

impl Drop for Sessions {
    fn drop(&mut self) {
        *self.0.stopped.lock().unwrap() = true;
        self.0.stop.notify_all();
        let ids: Vec<_> = self.0.sessions.lock().unwrap().keys().copied().collect();
        for id in ids {
            let _ = self.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_protocol::TerminalDelivery;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn id(n: u32) -> TabId {
        TabId::new(n).unwrap()
    }
    fn spec(cwd: &str) -> ShellSpec {
        ShellSpec {
            program: "/bin/sh".into(),
            args: vec![
                "-c".into(),
                "pwd; read answer; echo received:$answer".into(),
            ],
            cwd: Some(cwd.into()),
        }
    }
    fn collect(out: &SessionOutput, wanted: &str) -> std::collections::BTreeMap<TabId, String> {
        let mut text = BTreeMap::<TabId, String>::new();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            for delivery in out.take_batch() {
                if let TerminalDelivery::Output(bytes) = delivery.event {
                    text.entry(delivery.id)
                        .or_default()
                        .push_str(&String::from_utf8_lossy(&bytes));
                }
                out.ack(delivery.id, delivery.sequence);
            }
            if text.values().any(|text| text.contains(wanted)) {
                return text;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("missing {wanted}: {text:?}");
    }
    #[test]
    fn two_shells_have_independent_cwds_and_output() {
        let out = SessionOutput::new();
        let mut sessions = Sessions::new(out.clone());
        sessions
            .spawn(id(1), spec("/"), TerminalSize::new(80, 24).unwrap())
            .unwrap();
        sessions
            .spawn(id(2), spec("/private"), TerminalSize::new(80, 24).unwrap())
            .unwrap();
        sessions.write(id(1), b"alpha\n").unwrap();
        let result = collect(&out, "received:alpha");
        assert!(result[&id(1)].contains("received:alpha"));
        assert!(!result.get(&id(2)).is_some_and(|v| v.contains("alpha")));
        assert_eq!(sessions.observe(id(2)).cwd, Some(PathBuf::from("/private")));
    }
    #[test]
    fn failed_spawn_does_not_insert_session() {
        let out = SessionOutput::new();
        let mut sessions = Sessions::new(out.clone());
        let mut shell = spec("/");
        shell.program = "/missing/rake-shell".into();
        assert!(sessions
            .spawn(id(1), shell, TerminalSize::new(80, 24).unwrap())
            .is_err());
        assert!(!sessions.contains(id(1)));
        assert!(out.take_batch().is_empty());
    }
    #[test]
    fn closed_id_ignores_delayed_input_resize_and_exit() {
        let out = SessionOutput::new();
        let mut sessions = Sessions::new(out.clone());
        sessions
            .spawn(id(1), spec("/"), TerminalSize::new(80, 24).unwrap())
            .unwrap();
        sessions.remove(id(1)).unwrap();
        assert!(sessions.write(id(1), b"late").is_ok());
        assert!(sessions
            .resize(id(1), TerminalSize::new(10, 10).unwrap())
            .is_ok());
        assert!(out.take_batch().is_empty());
    }
    #[test]
    fn resize_targets_only_one_session() {
        let out = SessionOutput::new();
        let mut sessions = Sessions::new(out.clone());
        let size = TerminalSize::new(80, 24).unwrap();
        for n in [1, 2] {
            sessions
                .spawn(
                    id(n),
                    ShellSpec {
                        program: "/bin/sh".into(),
                        args: vec!["-c".into(), "read answer; stty size".into()],
                        cwd: None,
                    },
                    size,
                )
                .unwrap();
        }
        sessions
            .resize(id(1), TerminalSize::new(90, 30).unwrap())
            .unwrap();
        sessions.write(id(1), b"\n").unwrap();
        assert!(collect(&out, "30 90")[&id(1)].contains("30 90"));
        sessions.write(id(2), b"\n").unwrap();
        assert!(collect(&out, "24 80")[&id(2)].contains("24 80"));
    }
    #[test]
    fn remove_unblocks_backpressured_reader() {
        let out = SessionOutput::new();
        let mut sessions = Sessions::new(out.clone());
        sessions
            .spawn(
                id(1),
                ShellSpec {
                    program: "/bin/sh".into(),
                    args: vec!["-c".into(), "yes backpressure".into()],
                    cwd: None,
                },
                TerminalSize::new(80, 24).unwrap(),
            )
            .unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while out.outstanding(id(1)) < 256 * 1024 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        // Native PTY reads may be short: the event-slot bound can apply before
        // the byte bound. Either must be cancellable (the transport unit test
        // separately fills the entire byte budget with deterministic chunks).
        assert!(out.outstanding(id(1)) > 0);
        assert!(out.outstanding(id(1)) <= 256 * 1024);
        let start = Instant::now();
        sessions.remove(id(1)).unwrap();
        assert!(start.elapsed() < Duration::from_millis(300));
        assert_eq!(out.outstanding(id(1)), 0);
    }
}
