# Tabs acceptance evidence

Date: 2026-10-29. macOS only. Unchecked items are not claimed as verified.

## Automated

- [x] Stable IDs, selection/neighbours/cycling and manual/automatic naming.
- [x] Config validation, raw-byte paths, layout persistence/fallback and saved selection.
- [x] Real PTYs: initial/local cwd, isolated input/output/grid sizes and backpressure cancellation.
- [x] Native process tests: idle, foreground pipeline, background and stopped jobs;
  safe identity checks and cleanup of foreground/background/stopped jobs.
- [x] Workspace: new/home directory modes, last-tab new/hide/quit, spawn failure
  recovery, quit cancellation, confirmed-quit cleanup wait and layout preservation.
- [x] I/O failure does not bypass close/quit confirmation for surviving work.
- [x] Failed startup preserves the recoverable saved layout, including on quit.
- [x] Rename editing survives asynchronous selection/metadata round trips.
- [x] Real WebKit: independent screens, tagged OSC/input/paste, parse acks,
  selected-only fitting, top/bottom bar, overflow +, rename reset/cancel, text-safe
  titles, keyboard cycling/number intents and independent light/dark UI appearance.
- [x] Noisy background session does not starve a second shell's input in integration.

Latest automated results: **156 Rust tests** and **18 WebKit tests** pass. The
full Rust suite was also repeated three times after fixing test synchronization.

Commands: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --all-targets`, `cargo test --doc`, `npm ci`,
`npx playwright install webkit`, `npm test`. Use Node 22: Node 26 hung during
Playwright 1.56.1 archive extraction here; provisioning succeeded under Node 22.

## Native smoke evidence

- [x] User reviewed the running tabs build and reported it looks good on
  2026-10-30. This is general review, not evidence that every check below was run.

- [x] Actual AppKit main-menu Cmd+T/W/Q accelerators inspected, and each menu action
  invoked once in a temporary main-thread harness: NewTab, CloseActiveTab, Quit.
  This is not a physical-keyboard or dialog-appearance test.
- [x] Debug app launched under an isolated test HOME, wrote one-tab layout, and
  remained alive when a second invocation signalled it and exited 0.
- [x] Development DMG mounted; layout, plist and ad-hoc signature verified.
  Its bundle executable launched under an isolated HOME, persisted one tab and
  remained alive after a secondary invocation exited 0. Finder/Spotlight reopen
  of this new build still needs a hand check (the existing reopen path is retained).

## Hands-on acceptance still needed

- [ ] Physical Cmd+T/W/Q including held keys, Cmd+1–9, Ctrl+Tab variants and File menu.
- [ ] Visible + with 10+ tabs, both placements, rename/automatic title return.
- [ ] Native Cancel/Terminate dialogs visible above panel; safe default; repeated
  close/quit during modal; hide-on-focus-loss does not hide the prompt.
- [ ] Cmd+C/V and Ctrl+C; vim/top/SSH/server sessions across tabs.
- [ ] Close busy/idle tabs, cancel quit with several jobs, then confirm cleanup.
- [ ] Quit/relaunch with opt-in restoration; confirm fresh local shells, correct
  directories/names/selection, no SSH reconnection or process restoration.
- [ ] Last-tab new/hide/quit with real menu/hotkey/single-instance summons.
- [ ] 100,000 lines in an inactive tab while another remains interactive.
- [ ] Light/dark/live System appearance and unchanged Terminal profile colours.
- [ ] Hide/show, per-display native scaling, Spaces and full-screen apps with tabs.

## Boundaries

No confirmation guarantee for Force Quit, signals or macOS logout/shutdown.
Cleanup excludes deliberately detached/reparented daemons. Incomplete native
inspection prompts rather than declaring a tab idle. Directory/process observations
have normal races; later observations refresh saved local cwd. No release was published.

See [implementation decisions and review](tabs-implementation-notes.md) for the
cost-conscious execution approach, decisions and verified review fixes.
