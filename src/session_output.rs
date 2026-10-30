//! Bounded, cancellable per-tab output. Credit covers queued AND browser-in-flight bytes.

use crate::tabs::TabId;
use crate::terminal_protocol::{TerminalDelivery, TerminalEvent};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

const LIMIT: usize = 256 * 1024;
const CHUNK: usize = 8192;
const BATCH: usize = 64 * 1024;
const SLOTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDelivery {
    pub id: TabId,
    pub sequence: u32,
    pub event: TerminalDelivery,
}

#[derive(Debug)]
pub struct Closed;

#[derive(Default)]
struct Queue {
    pending: VecDeque<SessionDelivery>,
    flight: BTreeMap<u32, usize>,
    credit: usize,
    next: u32,
    ended: bool,
}

#[derive(Default)]
struct State {
    tabs: BTreeMap<TabId, Queue>,
    notified: bool,
    cursor: usize,
}

type Wake = Arc<dyn Fn() + Send + Sync>;
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    room: Condvar,
    wake: Mutex<Option<Wake>>,
}

#[derive(Clone, Default)]
pub struct SessionOutput(Arc<Shared>);

fn charge(event: &TerminalDelivery) -> usize {
    match event {
        TerminalDelivery::Output(bytes) => bytes.len().max(1),
        _ => 1,
    }
}

impl SessionOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_waker(&self, wake: impl Fn() + Send + Sync + 'static) {
        *self.0.wake.lock().unwrap() = Some(Arc::new(wake));
    }

    pub(crate) fn wake(&self) {
        let wake = self.0.wake.lock().unwrap().clone();
        if let Some(wake) = wake {
            wake();
        }
    }

    pub fn register(&self, id: TabId) {
        self.0.state.lock().unwrap().tabs.entry(id).or_default();
    }

    pub fn send(&self, id: TabId, event: TerminalEvent) -> Result<(), Closed> {
        match event {
            TerminalEvent::Output(bytes) => {
                for chunk in bytes.chunks(CHUNK) {
                    self.enqueue(id, TerminalDelivery::Output(chunk.to_vec()))?;
                }
                Ok(())
            }
            TerminalEvent::Exited { code } => self.enqueue(id, TerminalDelivery::Exited { code }),
            TerminalEvent::Failed(text) => self.enqueue(id, TerminalDelivery::Failed(text)),
            TerminalEvent::Paste(text) => self.enqueue(id, TerminalDelivery::Paste(text)),
        }
    }

    fn enqueue(&self, id: TabId, event: TerminalDelivery) -> Result<(), Closed> {
        let cost = charge(&event);
        let mut state = self.0.state.lock().unwrap();
        loop {
            let queue = state.tabs.get_mut(&id).ok_or(Closed)?;
            if queue.ended || queue.next == u32::MAX {
                return Err(Closed);
            }
            if queue.credit + cost <= LIMIT && queue.pending.len() + queue.flight.len() < SLOTS {
                queue.next += 1;
                queue.credit += cost;
                queue.ended = matches!(
                    event,
                    TerminalDelivery::Exited { .. } | TerminalDelivery::Failed(_)
                );
                queue.pending.push_back(SessionDelivery {
                    id,
                    sequence: queue.next,
                    event,
                });
                let wake = !state.notified;
                state.notified = true;
                drop(state);
                if wake {
                    self.wake();
                }
                return Ok(());
            }
            state = self.0.room.wait(state).unwrap();
        }
    }

    pub fn take_batch(&self) -> Vec<SessionDelivery> {
        let mut state = self.0.state.lock().unwrap();
        state.notified = false;
        let mut ids: Vec<_> = state.tabs.keys().copied().collect();
        if !ids.is_empty() {
            let offset = state.cursor % ids.len();
            ids.rotate_left(offset);
            state.cursor = state.cursor.wrapping_add(1);
        }
        let mut result = Vec::new();
        for id in ids {
            let queue = state.tabs.get_mut(&id).unwrap();
            let mut bytes = 0;
            while let Some(delivery) = queue.pending.front() {
                let cost = charge(&delivery.event);
                if bytes + cost > BATCH {
                    break;
                }
                bytes += cost;
                let delivery = queue.pending.pop_front().unwrap();
                queue.flight.insert(delivery.sequence, cost);
                result.push(delivery);
            }
        }
        let more = state.tabs.values().any(|q| !q.pending.is_empty());
        state.notified = more;
        drop(state);
        if more {
            self.wake();
        }
        result
    }

    pub fn retry(&self, batch: Vec<SessionDelivery>) {
        let mut state = self.0.state.lock().unwrap();
        for delivery in batch.into_iter().rev() {
            if let Some(queue) = state.tabs.get_mut(&delivery.id) {
                if queue.flight.remove(&delivery.sequence).is_some() {
                    queue.pending.push_front(delivery);
                }
            }
        }
        state.notified = false;
    }

    pub fn ack(&self, id: TabId, sequence: u32) {
        if let Some(queue) = self.0.state.lock().unwrap().tabs.get_mut(&id) {
            if let Some(cost) = queue.flight.remove(&sequence) {
                queue.credit -= cost;
            }
        }
        self.0.room.notify_all();
    }

    pub fn close(&self, id: TabId) {
        self.0.state.lock().unwrap().tabs.remove(&id);
        self.0.room.notify_all();
    }

    pub fn outstanding(&self, id: TabId) -> usize {
        self.0
            .state
            .lock()
            .unwrap()
            .tabs
            .get(&id)
            .map_or(0, |q| q.credit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn id(n: u32) -> TabId {
        TabId::new(n).unwrap()
    }

    #[test]
    fn tabs_keep_independent_order_and_exit_after_output() {
        let out = SessionOutput::new();
        out.register(id(1));
        out.register(id(2));
        out.send(id(1), TerminalEvent::Output(b"one".to_vec()))
            .unwrap();
        out.send(id(2), TerminalEvent::Output(b"two".to_vec()))
            .unwrap();
        out.send(id(1), TerminalEvent::Exited { code: Some(0) })
            .unwrap();
        let batch = out.take_batch();
        let one: Vec<_> = batch.iter().filter(|d| d.id == id(1)).collect();
        assert!(matches!(&one[0].event, TerminalDelivery::Output(b) if b == b"one"));
        assert!(matches!(
            &one[1].event,
            TerminalDelivery::Exited { code: Some(0) }
        ));
        assert!(one[0].sequence < one[1].sequence);
    }

    #[test]
    fn removed_tab_wakes_blocked_sender() {
        let out = SessionOutput::new();
        out.register(id(1));
        out.send(id(1), TerminalEvent::Output(vec![42; 256 * 1024]))
            .unwrap();
        let (tx, rx) = mpsc::channel();
        let writer = out.clone();
        let worker = std::thread::spawn(move || {
            tx.send(writer.send(id(1), TerminalEvent::Output(vec![1])))
                .unwrap();
        });
        assert!(rx.recv_timeout(Duration::from_millis(30)).is_err());
        out.close(id(1));
        assert!(rx.recv_timeout(Duration::from_secs(1)).unwrap().is_err());
        worker.join().unwrap();
    }

    #[test]
    fn invalid_or_duplicate_ack_does_not_release_credit() {
        let out = SessionOutput::new();
        out.register(id(1));
        out.send(id(1), TerminalEvent::Output(vec![1; 8192]))
            .unwrap();
        let first = out.take_batch().remove(0);
        out.ack(id(1), first.sequence + 1);
        assert_eq!(out.outstanding(id(1)), 8192);
        out.ack(id(1), first.sequence);
        out.ack(id(1), first.sequence);
        assert_eq!(out.outstanding(id(1)), 0);
    }

    #[test]
    fn one_noisy_tab_cannot_starve_another() {
        let out = SessionOutput::new();
        out.register(id(1));
        out.register(id(2));
        out.send(id(1), TerminalEvent::Output(vec![1; 256 * 1024]))
            .unwrap();
        out.send(id(2), TerminalEvent::Output(vec![2])).unwrap();
        let batch = out.take_batch();
        assert!(batch.iter().any(|d| d.id == id(2)));
        let bytes: usize = batch
            .iter()
            .filter_map(|d| {
                if let TerminalDelivery::Output(b) = &d.event {
                    (d.id == id(1)).then_some(b.len())
                } else {
                    None
                }
            })
            .sum();
        assert_eq!(bytes, 64 * 1024);
        assert_eq!(out.outstanding(id(1)), 256 * 1024);
    }

    #[test]
    fn late_removed_id_events_are_ignored() {
        let out = SessionOutput::new();
        out.register(id(1));
        out.close(id(1));
        assert!(out
            .send(id(1), TerminalEvent::Exited { code: Some(0) })
            .is_err());
        out.ack(id(1), 1);
        assert!(out.take_batch().is_empty());
    }

    #[test]
    fn retry_keeps_order_and_credit() {
        let out = SessionOutput::new();
        out.register(id(1));
        out.send(id(1), TerminalEvent::Output(b"abc".to_vec()))
            .unwrap();
        let batch = out.take_batch();
        out.retry(batch.clone());
        assert_eq!(out.take_batch(), batch);
        assert_eq!(out.outstanding(id(1)), 3);
    }
}
