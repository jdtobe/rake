//! One confirmation policy for every close/quit control; never mutates sessions.
use crate::process::JobState;
use crate::tabs::{TabId, Tabs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseIntent {
    Tab(TabId),
    Quit,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseDecision {
    Ignore,
    Confirm { token: u64, affected: Vec<TabId> },
    Proceed { intent: CloseIntent },
}
struct Pending {
    token: u64,
    intent: CloseIntent,
    affected: Vec<TabId>,
}
#[derive(Default)]
pub struct ClosePolicy {
    next: u64,
    pending: Option<Pending>,
}

impl ClosePolicy {
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn request(
        &mut self,
        intent: CloseIntent,
        tabs: &Tabs,
        jobs: &[(TabId, JobState)],
    ) -> CloseDecision {
        if self.pending() {
            return CloseDecision::Ignore;
        }
        if matches!(intent,CloseIntent::Tab(id) if tabs.get(id).is_none()) {
            return CloseDecision::Ignore;
        }
        let affected = Self::affected(intent, tabs, jobs);
        if affected.is_empty() {
            CloseDecision::Proceed { intent }
        } else {
            self.confirm(intent, affected)
        }
    }
    fn affected(intent: CloseIntent, tabs: &Tabs, jobs: &[(TabId, JobState)]) -> Vec<TabId> {
        tabs.ordered()
            .iter()
            .filter(|tab| {
                (intent == CloseIntent::Quit || intent == CloseIntent::Tab(tab.id))
                    && !tab.ended
                    && !matches!(
                        jobs.iter()
                            .find(|(id, _)| *id == tab.id)
                            .map(|(_, state)| *state),
                        Some(JobState::Idle | JobState::Ended)
                    )
            })
            .map(|tab| tab.id)
            .collect()
    }
    fn confirm(&mut self, intent: CloseIntent, affected: Vec<TabId>) -> CloseDecision {
        let Some(token) = self.next.checked_add(1) else {
            return CloseDecision::Ignore;
        };
        self.next = token;
        self.pending = Some(Pending {
            token,
            intent,
            affected: affected.clone(),
        });
        CloseDecision::Confirm { token, affected }
    }
    pub fn resolve(
        &mut self,
        token: u64,
        confirmed: bool,
        jobs: &[(TabId, JobState)],
        tabs: &Tabs,
    ) -> CloseDecision {
        if !self.pending.as_ref().is_some_and(|p| p.token == token) {
            return CloseDecision::Ignore;
        }
        let pending = self.pending.take().unwrap();
        if !confirmed {
            return CloseDecision::Ignore;
        }
        if matches!(pending.intent,CloseIntent::Tab(id) if tabs.get(id).is_none()) {
            return CloseDecision::Ignore;
        }
        let affected = Self::affected(pending.intent, tabs, jobs);
        if affected.iter().any(|id| !pending.affected.contains(id)) {
            return self.confirm(pending.intent, affected);
        }
        CloseDecision::Proceed {
            intent: pending.intent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tabs() -> (Tabs, TabId, TabId) {
        let mut tabs = Tabs::new();
        let a = tabs.reserve_id().unwrap();
        let b = tabs.reserve_id().unwrap();
        tabs.insert(a, "/".into(), Default::default());
        tabs.insert(b, "/".into(), Default::default());
        (tabs, a, b)
    }
    #[test]
    fn idle_and_ended_close_without_prompt() {
        let (tabs, a, b) = tabs();
        let mut policy = ClosePolicy::default();
        assert_eq!(
            policy.request(CloseIntent::Tab(a), &tabs, &[(a, JobState::Idle)]),
            CloseDecision::Proceed {
                intent: CloseIntent::Tab(a)
            }
        );
        assert_eq!(
            policy.request(CloseIntent::Tab(b), &tabs, &[(b, JobState::Ended)]),
            CloseDecision::Proceed {
                intent: CloseIntent::Tab(b)
            }
        );
    }
    #[test]
    fn busy_and_unknown_prompt() {
        let (tabs, a, _) = tabs();
        for state in [JobState::Busy, JobState::Unknown] {
            assert!(
                matches!(ClosePolicy::default().request(CloseIntent::Tab(a),&tabs,&[(a,state)]),CloseDecision::Confirm{affected,..} if affected==vec![a])
            );
        }
    }
    #[test]
    fn quit_lists_all_at_risk_tabs_once() {
        let (tabs, a, b) = tabs();
        assert!(
            matches!(ClosePolicy::default().request(CloseIntent::Quit,&tabs,&[(a,JobState::Busy),(b,JobState::Unknown)]),CloseDecision::Confirm{affected,..} if affected==vec![a,b])
        );
    }
    #[test]
    fn cancel_changes_nothing() {
        let (tabs, a, _) = tabs();
        let mut policy = ClosePolicy::default();
        let jobs = [(a, JobState::Busy)];
        let CloseDecision::Confirm { token, .. } =
            policy.request(CloseIntent::Tab(a), &tabs, &jobs)
        else {
            panic!()
        };
        assert_eq!(
            policy.resolve(token, false, &jobs, &tabs),
            CloseDecision::Ignore
        );
        assert!(!policy.pending());
        assert_eq!(tabs.ordered().len(), 2);
    }
    #[test]
    fn duplicate_pending_request_is_ignored() {
        let (tabs, a, _) = tabs();
        let mut policy = ClosePolicy::default();
        policy.request(CloseIntent::Tab(a), &tabs, &[(a, JobState::Busy)]);
        assert_eq!(
            policy.request(CloseIntent::Quit, &tabs, &[]),
            CloseDecision::Ignore
        );
        assert!(policy.pending());
    }
    #[test]
    fn stale_confirmation_cannot_close_new_tabs() {
        let (tabs, a, _) = tabs();
        let mut policy = ClosePolicy::default();
        let jobs = [(a, JobState::Busy)];
        let CloseDecision::Confirm { token, .. } =
            policy.request(CloseIntent::Tab(a), &tabs, &jobs)
        else {
            panic!()
        };
        assert_eq!(
            policy.resolve(token + 1, true, &jobs, &tabs),
            CloseDecision::Ignore
        );
        assert!(policy.pending());
        assert_eq!(
            policy.resolve(token, true, &jobs, &tabs),
            CloseDecision::Proceed {
                intent: CloseIntent::Tab(a)
            }
        );
        assert_eq!(
            policy.resolve(token, true, &jobs, &tabs),
            CloseDecision::Ignore
        );
    }
    #[test]
    fn quit_rechecks_new_or_changed_jobs() {
        let (tabs, a, b) = tabs();
        let mut policy = ClosePolicy::default();
        let CloseDecision::Confirm { token, .. } = policy.request(
            CloseIntent::Quit,
            &tabs,
            &[(a, JobState::Busy), (b, JobState::Idle)],
        ) else {
            panic!()
        };
        let CloseDecision::Confirm {
            token: new,
            affected,
        } = policy.resolve(
            token,
            true,
            &[(a, JobState::Busy), (b, JobState::Busy)],
            &tabs,
        )
        else {
            panic!()
        };
        assert_ne!(token, new);
        assert_eq!(affected, vec![a, b]);
    }
}
