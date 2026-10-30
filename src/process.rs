//! Local process inspection. Detached/reparented daemons outside the captured
//! session tree are not cleanup targets. Names and terminal output are not evidence.
use std::path::PathBuf;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::{identity, observe, snapshot, terminate};
#[cfg(all(test, target_os = "macos"))]
pub(crate) use macos::{is_live, is_stopped};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub started_at: (u64, u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Idle,
    Busy,
    Unknown,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionObservation {
    pub cwd: Option<PathBuf>,
    pub jobs: JobState,
}

#[derive(Clone, Copy, Debug)]
struct ProcessInfo {
    identity: ProcessIdentity,
    parent: u32,
    group: u32,
    zombie: bool,
}

fn classify(
    root: Option<ProcessInfo>,
    processes: &[ProcessInfo],
    complete: bool,
    foreground: Option<u32>,
) -> JobState {
    let Some(root) = root else {
        return JobState::Unknown;
    };
    if !complete || foreground.is_none() {
        return JobState::Unknown;
    }
    if root.zombie {
        return JobState::Ended;
    }
    let mut descendants = vec![root.identity.pid];
    loop {
        let previous = descendants.len();
        for process in processes {
            if descendants.contains(&process.parent) && !descendants.contains(&process.identity.pid)
            {
                descendants.push(process.identity.pid);
            }
        }
        if descendants.len() == previous {
            break;
        }
    }
    if processes
        .iter()
        .any(|p| descendants.contains(&p.identity.pid) && !p.zombie)
        || foreground != Some(root.group)
    {
        JobState::Busy
    } else {
        JobState::Idle
    }
}

fn safe_target(expected: ProcessIdentity, actual: Option<ProcessIdentity>, own_pid: u32) -> bool {
    expected.pid > 1
        && expected.pid <= i32::MAX as u32
        && expected.pid != own_pid
        && actual == Some(expected)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn identity(_: u32) -> Option<ProcessIdentity> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pid: u32, parent: u32, zombie: bool) -> ProcessInfo {
        ProcessInfo {
            identity: ProcessIdentity {
                pid,
                started_at: (1, 0),
            },
            parent,
            group: 10,
            zombie,
        }
    }

    #[test]
    fn inspection_failure_is_unknown() {
        assert_eq!(classify(None, &[], false, Some(10)), JobState::Unknown);
        assert_eq!(
            classify(Some(entry(10, 1, false)), &[], false, Some(10)),
            JobState::Unknown
        );
    }

    #[test]
    fn reused_pid_is_not_a_kill_target() {
        let old = entry(10, 1, false).identity;
        let mut new = old;
        new.started_at.1 += 1;
        assert!(!safe_target(old, Some(new), 99));
        assert!(!safe_target(old, Some(old), 10));
        assert!(safe_target(old, Some(old), 99));
    }

    #[test]
    fn zombie_does_not_count_as_live_job() {
        assert_eq!(
            classify(
                Some(entry(10, 1, false)),
                &[entry(11, 10, true)],
                true,
                Some(10)
            ),
            JobState::Idle
        );
    }

    #[test]
    fn unrelated_same_name_process_is_not_descendant() {
        assert_eq!(
            classify(
                Some(entry(10, 1, false)),
                &[entry(11, 99, false)],
                true,
                Some(10)
            ),
            JobState::Idle
        );
    }
}
