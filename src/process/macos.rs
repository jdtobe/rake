//! The only native inspection/signalling boundary. No negative-PID group kills:
//! enumerate group members through ancestry and signal each verified identity,
//! avoiding collateral kills if a group ID is reused or gains unrelated members.
use super::*;
use portable_pty::ChildKiller;
use std::ffi::OsString;
use std::mem::{size_of, zeroed};
use std::os::unix::ffi::OsStringExt;
use std::time::{Duration, Instant};

fn info(pid: u32) -> Result<Option<ProcessInfo>, String> {
    if pid == 0 || pid > i32::MAX as u32 {
        return Err("invalid process ID".into());
    }
    // SAFETY: initialized buffer with the exact libproc structure size.
    let mut data: libc::proc_bsdinfo = unsafe { zeroed() };
    let size = size_of::<libc::proc_bsdinfo>() as i32;
    let count = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut data as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    if count != size {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        return Err(format!("cannot inspect PID {pid}: {error}"));
    }
    if data.pbi_pid != pid {
        return Err("process identity changed during inspection".into());
    }
    Ok(Some(ProcessInfo {
        identity: ProcessIdentity {
            pid,
            started_at: (data.pbi_start_tvsec, data.pbi_start_tvusec),
        },
        parent: data.pbi_ppid,
        group: data.pbi_pgid,
        zombie: data.pbi_status == libc::SZOMB,
    }))
}

pub(crate) fn identity(pid: u32) -> Option<ProcessIdentity> {
    info(pid).ok().flatten().map(|p| p.identity)
}

#[cfg(test)]
pub(crate) fn is_stopped(id: ProcessIdentity) -> bool {
    // SAFETY: initialized native buffer of precisely the required size.
    let mut data: libc::proc_bsdinfo = unsafe { zeroed() };
    let size = size_of::<libc::proc_bsdinfo>() as i32;
    let count = unsafe {
        libc::proc_pidinfo(
            id.pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut data as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    count == size
        && (data.pbi_start_tvsec, data.pbi_start_tvusec) == id.started_at
        && data.pbi_status == libc::SSTOP
}

pub(crate) fn is_live(id: ProcessIdentity) -> Result<bool, String> {
    Ok(info(id.pid)?.is_some_and(|p| p.identity == id && !p.zombie))
}

fn children(pid: u32) -> Result<Vec<u32>, String> {
    // A full buffer is ambiguous (including exact fit), never silently truncated.
    let mut pids = vec![0_u32; 4096];
    let bytes = (pids.len() * size_of::<u32>()) as i32;
    // SAFETY: errno is thread local; buffer points to bytes writable by libproc.
    let count = unsafe {
        *libc::__error() = 0;
        libc::proc_listchildpids(pid as i32, pids.as_mut_ptr().cast(), bytes)
    };
    // Unlike proc_listpids, this returns a PID count, not a byte count.
    if count < 0
        || count as usize >= pids.len()
        || (count == 0 && std::io::Error::last_os_error().raw_os_error() != Some(0))
    {
        return Err(format!("incomplete child enumeration for PID {pid}"));
    }
    pids.truncate(count as usize);
    pids.sort_unstable();
    Ok(pids)
}

fn tree(root: ProcessIdentity) -> Result<Vec<ProcessInfo>, String> {
    let deadline = Instant::now() + Duration::from_millis(100);
    let Some(first) = info(root.pid)? else {
        return Ok(Vec::new());
    };
    if first.identity != root {
        return Ok(Vec::new());
    }
    let mut found = vec![first];
    let mut index = 0;
    while index < found.len() {
        if Instant::now() >= deadline {
            return Err("process inspection deadline exceeded".into());
        }
        if found.len() > 4096 {
            return Err("session process tree exceeds inspection limit".into());
        }
        let parent = found[index];
        let before = children(parent.identity.pid)?;
        for &pid in &before {
            if Instant::now() >= deadline {
                return Err("process inspection deadline exceeded".into());
            }
            let child = info(pid)?.ok_or("child disappeared during inspection")?;
            if child.parent != parent.identity.pid || found.iter().any(|p| p.identity.pid == pid) {
                return Err("process ancestry changed during inspection".into());
            }
            found.push(child);
        }
        if children(parent.identity.pid)? != before
            || identity(parent.identity.pid) != Some(parent.identity)
        {
            return Err("process tree changed during inspection".into());
        }
        index += 1;
    }
    if identity(root.pid) != Some(root) {
        return Err("shell identity changed".into());
    }
    Ok(found)
}

pub(crate) fn snapshot(root: ProcessIdentity) -> Result<Vec<ProcessIdentity>, String> {
    Ok(tree(root)?.into_iter().map(|p| p.identity).collect())
}

fn cwd(id: ProcessIdentity) -> Option<PathBuf> {
    // SAFETY: correctly sized, initialized libproc buffer; path copied as bytes.
    let mut data: libc::proc_vnodepathinfo = unsafe { zeroed() };
    let size = size_of::<libc::proc_vnodepathinfo>() as i32;
    let count = unsafe {
        libc::proc_pidinfo(
            id.pid as i32,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut data as *mut libc::proc_vnodepathinfo).cast(),
            size,
        )
    };
    if count != size || identity(id.pid) != Some(id) {
        return None;
    }
    let bytes: Vec<u8> = data
        .pvi_cdir
        .vip_path
        .iter()
        .flatten()
        .map(|c| *c as u8)
        .take_while(|c| *c != 0)
        .collect();
    if bytes.is_empty() {
        None
    } else {
        Some(OsString::from_vec(bytes).into())
    }
}

pub(crate) fn observe(id: ProcessIdentity, fd: Option<i32>) -> SessionObservation {
    let cwd = cwd(id);
    let foreground = fd.and_then(|fd| {
        // SAFETY: borrowed master descriptor remains open throughout observation.
        let group = unsafe { libc::tcgetpgrp(fd) };
        (group > 0).then_some(group as u32)
    });
    let jobs = match tree(id) {
        Ok(processes) if !processes.is_empty() => classify(
            processes.first().copied(),
            &processes[1..],
            true,
            foreground,
        ),
        // Missing/reused identities are uncertain, not proof of session completion.
        _ => JobState::Unknown,
    };
    SessionObservation { cwd, jobs }
}

fn signal(id: ProcessIdentity, signal: i32) -> Result<(), String> {
    let Some(current) = info(id.pid)? else {
        return Ok(());
    };
    if current.identity != id || current.zombie {
        return Ok(());
    }
    // SAFETY: getpgrp has no preconditions. Never target the application's group.
    if !safe_target(id, Some(current.identity), std::process::id())
        || current.group == unsafe { libc::getpgrp() } as u32
    {
        return Err(format!("refusing unsafe signal target {}", id.pid));
    }
    // SAFETY: positive, identity-checked PID; no process-group broadcast.
    if unsafe { libc::kill(id.pid as i32, signal) } != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(format!("signal {}: {error}", id.pid));
        }
    }
    Ok(())
}

pub(crate) fn terminate(
    root: ProcessIdentity,
    captured: Result<Vec<ProcessIdentity>, String>,
    mut killer: Box<dyn ChildKiller + Send + Sync>,
) -> Result<(), String> {
    let cleanup_deadline = Instant::now() + Duration::from_secs(2);
    let mut errors = Vec::new();
    let mut targets = match captured {
        Ok(targets) => targets,
        Err(error) => {
            errors.push(error);
            vec![root]
        }
    };
    // Children before shell; keep captured identities after reparenting.
    targets.reverse();
    for stage in [libc::SIGHUP, libc::SIGTERM, libc::SIGKILL] {
        // Pick up jobs forked before teardown while the shell is still inspectable.
        if is_live(root).unwrap_or(false) {
            match snapshot(root) {
                Ok(new) => {
                    for id in new {
                        if !targets.contains(&id) {
                            targets.push(id);
                        }
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        for &id in &targets {
            if Instant::now() >= cleanup_deadline {
                errors.push("process cleanup deadline exceeded".into());
                break;
            }
            if let Err(error) = signal(id, stage) {
                errors.push(error);
            }
            if stage == libc::SIGHUP {
                if let Err(error) = signal(id, libc::SIGCONT) {
                    errors.push(error);
                }
            }
        }
        // Use portable-pty's tracked shell killer only after native identity/group
        // checks, and only at the HUP stage (its Unix implementation sends HUP).
        if stage == libc::SIGHUP {
            if let Ok(Some(p)) = info(root.pid) {
                if safe_target(root, Some(p.identity), std::process::id())
                    && p.group != unsafe { libc::getpgrp() } as u32
                    && !p.zombie
                {
                    if let Err(error) = killer.kill() {
                        if error.raw_os_error() != Some(libc::ESRCH) {
                            errors.push(error.to_string());
                        }
                    }
                }
            }
        }
        let deadline = Instant::now() + Duration::from_millis(400);
        while Instant::now() < deadline && Instant::now() < cleanup_deadline {
            if targets.iter().all(|id| matches!(is_live(*id), Ok(false))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if targets.iter().all(|id| matches!(is_live(*id), Ok(false))) {
            break;
        }
    }
    for id in targets {
        if Instant::now() >= cleanup_deadline {
            errors.push("could not verify all processes before cleanup deadline".into());
            break;
        }
        match is_live(id) {
            Ok(false) => {}
            Ok(true) => errors.push(format!("PID {} survived cleanup deadline", id.pid)),
            Err(error) => errors.push(error),
        }
    }
    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
