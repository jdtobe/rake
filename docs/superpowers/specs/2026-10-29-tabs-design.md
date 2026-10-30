# Tabs as independent workspaces

Date: 2026-10-29
Status: Approved by user on 2026-10-29

## Intent and priority

Tabs are the next product milestone, ahead of a general Settings UI and animation
polish. They support different servers, applications, pipeline stages and files.
Each tab owns a shell, terminal screen and scrollback. Switching tabs or hiding
Rake must not interrupt work. Keep the existing Rust, tao, wry and xterm.js stack.

This is a macOS implementation; do not claim support for other platforms.

## Agreed behaviour

- Tab bar at the bottom by default; top placement configurable. Keep it visible
  with one tab. It occupies space inside the existing panel, reducing terminal
  rows, not increasing the configured panel height.
- An obvious **+** button creates a tab and stays reachable when tabs overflow.
  Tabs remain a single scrolling row; names truncate. Active tab has an amber
  accent and is automatically scrolled into view.
- Cmd+T creates, Cmd+W closes, Cmd+1–9 selects, Ctrl+Tab and Ctrl+Shift+Tab cycle.
  File → New Tab and File → Close Tab are equivalent. Tab × uses the same close
  path. Preserve Cmd+C/Cmd+V and normal terminal key handling.
- New tabs start fresh local login shells in the current tab's local working
  directory by default; starting in home is configurable. Never reconnect SSH.
- Automatic name: shell/program title updates (OSC 0/2); directory name if no
  title has been received. Manual names override automatic naming. Clearing a
  manual name restores automatic naming. Double-click a name to edit it.
- Closing a tab with a running application asks before terminating it. Idle
  shells and completed sessions close without a prompt. Quitting asks once if
  any tab has running applications. Cancellation changes nothing.
- Startup defaults to one fresh tab. Optional restoration recreates the previous
  tab order, names and local directories with fresh shells. No running commands,
  scrollback or command history are restored by Rake.
- Closing the last tab defaults to creating a fresh tab. Alternatives are hiding
  the panel or quitting. After hiding an empty panel, the next summon creates a
  fresh tab. Reopening the app/single-instance show request counts as summoning.
- UI appearance supports System (default), Light and Dark. Terminal colours and
  font remain independently supplied by the macOS Terminal profile. No profile
  management or terminal theme switching in this milestone.
- Settings initially live in the existing TOML config and apply on restart.

Normal shell history files still behave as configured by the user's shell;
"no history restored" means Rake does not capture or replay history.

## Approved builder decisions

### Session model and ownership

Add a pure Rust tab model with monotonic session IDs (not reusable indices), order,
active ID, naming mode, last known local directory and lifecycle state. A separate
session manager owns PTYs; main.rs stays event wiring, not a second state machine.
One xterm instance/container per session in the existing single webview.

All session-scoped events and IPC messages carry an ID. UI commands are validated
in Rust; Rust is authoritative about creation, selection, names and removal.
Discard late output/exit/resize/paste/title events for removed IDs. An asynchronous
paste response must return to its originating session, not whichever tab is now
selected. Global document-ready and UI appearance are not session-scoped.

Switching changes visibility and focus, not the terminal instance. Inactive xterms
continue parsing output so their screens stay current, using existing bounded
scrollback. Use WebGL only for the active terminal; inactive sessions retain their
terminal buffers with the non-WebGL renderer. Dispose/recreate the addon on switch;
retain the current fallback when WebGL is unavailable. Do not allocate one WebGL
context per tab. Fit the selected terminal after switching and after bar placement,
panel-size or display-scale changes. Never resize a PTY to a zero-size hidden DOM
container; inactive sessions keep their last valid grid until selected again.

Preserve output ordering and output-before-exit separately for every session.
Use bounded output batches/queues with backpressure rather than an unbounded
inactive-tab backlog; service sessions fairly so a noisy background command does
not starve typing or tab switching. Closing must release any blocked output worker.

### Local directories and process safety

Retain shell PID and PTY information at spawn. A small macOS-only process adapter
reads the local shell's cwd via native process APIs. This works without modifying
shell startup files and ignores remote OSC directory reports. Capture cwd on new
tab and before persisting session state; refresh periodically for crash recovery.
If unavailable, use the last known local cwd, then home. If a saved directory is
missing or inaccessible, open home and show a non-blocking notice.

Inspect the PTY foreground process group and the shell's live descendants for
foreground or background applications; a live shell alone does not mean busy.
SSH, editors, servers, pipelines, stopped jobs and background jobs count as work.
Session exit means not busy. If process inspection is unavailable/ambiguous, ask
before closing rather than risk destroying work. Busy indicators may use cached
status, but close/quit performs a fresh check. Shell/process observations can race;
do not promise infallible detection of a process started at the exact close instant.

Native confirmation dialogs use explicit Cancel and Terminate actions, listing
affected tab names for quit. A pending dialog prevents duplicate close requests;
do not let focus loss auto-hide the panel while asking. If tabs change during a
quit check, revalidate the set before termination. Never interpret a shell title
or output activity as proof that a job is running or idle.

User-initiated close/quit cleans up the shell and its associated job groups, not
merely the root shell PID. Avoid signalling unrelated processes or stale/reused
PIDs. Verify this with foreground and background jobs. Force Quit and system kill
cannot provide a confirmation guarantee.

All in-app quit routes (tray Quit, application Quit/Cmd+Q, and last-tab quit) go
through one coordinator, replacing the current predefined Quit item that exits
directly. Native OS termination integration must be investigated during planning;
if it cannot be safely intercepted through tao, record that limitation explicitly
rather than claiming all system shutdown paths are covered.

### Tab actions and edge cases

New tab selects the new session only after successful spawn. Failure leaves the
existing tab and selection intact and reports the error. Failure to create a
replacement for the last tab leaves an empty panel with a working +/New Tab and a
visible retryable error, not a crash or accidental quit.

Closing the active tab selects the tab to its right, otherwise its left neighbour.
Closing an inactive tab preserves selection. Cycling wraps. Cmd+1–8 selects those
positions; Cmd+9 selects the last tab. Missing positions do nothing. Drag reordering
is wanted follow-up work, deferred from this milestone; preserve creation order
apart from removals. IDs remain independent of order so reordering can be added
without changing session identity.

Manual rename accepts text, trims surrounding whitespace and treats empty text as
automatic. Escape cancels; Enter saves. Render title strings as text, never HTML;
strip control characters and cap their length. Display names do not become shell
commands. Existing completed-shell behaviour remains: show process-ended message
and retain the tab until the user closes it. Tab creation does not replay commands.

### Config and saved layout

Proposed flat config keys, consistent with the current config:

```toml
tab_position = "bottom"       # bottom | top
new_tab_directory = "current" # current | home
startup_tabs = "new"          # new | restore
last_tab_action = "new"       # new | hide | quit
appearance = "system"         # system | light | dark
```

Validate enums and keep existing strict unknown-key handling. Defaults preserve
current users' startup experience (one fresh login shell).

Persist layout separately from config in
`~/Library/Application Support/Rake/tabs.json`, using a versioned schema and atomic
replacement. Store order, last selected position, local paths, naming mode and
last displayed automatic title; no output, history, environment or commands.
Preserve custom names exactly. A saved automatic title is provisional until the
fresh shell supplies a title; it is not converted into a manual name. Save layout
on structural/rename changes and periodically on cwd changes. Capture it before
quit, not after deleting sessions. Do not overwrite the saved layout when quit is
cancelled. Invalid/unsupported layout: warn and open one fresh tab. Empty saved
layout likewise becomes one fresh tab on startup. Save the layout even in new-tab
startup mode so opting into restoration can use the latest previous run.

System appearance follows changes live using the native/webview system colour
scheme; explicit Light/Dark overrides that for Rake controls only. Set dialog and
menu appearance consistently where AppKit supports it. Use accessible contrasting
text/focus states in both themes; the terminal canvas is unchanged.

## Non-goals

General Settings window, shell profiles, split panes, remote reconnect, session
process resurrection, scrollback persistence, custom terminal palettes, tab drag
reordering and detached windows. Release publication is a separate operation and
has not been authorised by approving this feature design.

## Verification

- Pure model tests: IDs, selection, cycling, last-tab actions, title precedence,
  sanitisation, cancelled actions and session restore policies.
- Config/schema tests: defaults, invalid values, round-trip, missing/corrupt/newer
  layouts, atomic writes and missing-directory fallback.
- PTY integration: initial cwd; independent input/output/resize/exit; idle shell,
  foreground job, pipeline, background job and process-inspection failure;
  cancelling prompts leaves processes alive; confirming cleans up jobs.
- Browser-side tests exercise real tab DOM and keyboard handling, automatic OSC
  titles, overflow/+ reachability, rename reset, hidden grids and appearance,
  rather than asserting that source strings happen to exist.
- Routing/backpressure tests: late removed-session events, paste target, per-tab
  output ordering, heavy inactive output and disposal while blocked.
- macOS hand checks: vim/top/SSH/server sessions, both tab placements, both UI
  appearances plus live system switch, all menu and keyboard close/quit routes,
  cancel/terminate, single-instance show from an empty hidden panel, restart
  restoration, native per-display scaling, and the existing terminal acceptance
  checklist. Run fmt, clippy and all Rust tests; keep docs honest about unverified
  cases and do not claim other platforms work.

## Design workflow checklist

- [x] Gather behaviour and priority with user.
- [x] Show interactive placement mockups; bottom default accepted.
- [x] Inspect existing PTY/webview/protocol architecture.
- [x] Draft complete design with builder decisions marked.
- [x] Independent builder review: no important unanswered questions.
- [x] Present design to user for approval.
- [x] After approval, write implementation plan before product changes
  ([plan](../plans/2026-10-29-tabs.md)).
