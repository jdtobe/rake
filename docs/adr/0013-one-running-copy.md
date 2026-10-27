# 13. One running copy, and a second launch shows the panel

Date: 2026-10-27 · Status: Accepted

## Context
Launching Rake twice (the binary directly, or `open -n`) gave two copies: two login shells, two event loops, both claiming the same configuration and hotkey. macOS itself reuses an existing `.app` for Finder and Spotlight launches, but that only activates it; with a menu-bar-only app whose panel may be hidden, nothing visible happens. The wanted behaviour is one process: a second launch brings the existing panel forward and exits.

## Decision
- The first copy takes an exclusive kernel lock on `~/Library/Application Support/Rake/instance.lock` with `File::try_lock` and keeps it for its lifetime. The lock, not the file, is authoritative: exit, crash, force quit or reboot releases it, so a leftover file never blocks a start.
- The locked copy listens on a Unix socket beside the lock (`instance.sock`), removing any stale socket first. A second launch fails the lock, connects (retrying briefly while the first starts), asks the panel to show, and exits with status 0.
- The running copy handles that request and macOS's reopen event through one path: a hidden panel slides in on the display under the pointer, a visible one takes focus.
- The development binary shares the lock, so `cargo run` defers to an installed Rake; quit it first to run a dev build.

## Consequences
- A running copy that hangs keeps the lock. A second launch then exits without showing anything. Stealing the lock would risk two instances, and quitting the hung app is the real fix.
- The state directory is not configurable and does not use the bundle identifier: the binary and the app are the same app for this purpose.
- The socket is per user and local to the machine; nothing is exposed over the network.
