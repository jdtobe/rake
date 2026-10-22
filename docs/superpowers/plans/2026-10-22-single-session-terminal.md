# Single-Session Terminal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Rake's placeholder panel with one persistent, immediately focused macOS login-shell session rendered by bundled xterm.js.

**Architecture:** `portable-pty` owns the shell and emits ordered binary events through the existing `tao` loop. A `wry` WKWebView hosts offline xterm.js assets; a tested Rust protocol parses webview requests, buffers output until JavaScript is ready, and generates binary-safe delivery scripts. `DropDown` continues to own placement and animation while delegating terminal presentation to `TerminalView`.

**Tech Stack:** Rust 2021, `tao` 0.37, `wry` 0.57, `portable-pty` 0.9, `serde_json`, `base64` 0.23, xterm.js 6.0.0, `@xterm/addon-fit` 0.11.0, WKWebView

**Spec:** `docs/superpowers/specs/2026-10-22-rake-terminal-design.md`

## Global Constraints

- Complete the Rake rename plan before this plan; all imports and paths here use `rake`.
- One default macOS login-shell session starts during direct application launch.
- Direct launch shows and focuses Rake; hiding never terminates or recreates the session.
- xterm.js supplies both rendering and terminal emulation; do not add another emulator core.
- All web assets are bundled locally with license notices; no runtime network dependency or unrestricted navigation.
- Preserve arbitrary PTY bytes and their order without assuming read chunks are UTF-8.
- Shell exit leaves terminal contents visible and shows a process-ended message; no automatic restart.
- Tabs, profiles, Settings, launch-at-login, packaging, and cross-platform work are deferred.
- Keep `main.rs` as wiring, pure logic tested in library modules, and macOS glue small.

## Review Focus

- `$SHELL` missing or empty must select `/bin/zsh`; pinned by Task 2 shell-spec tests.
- Malformed IPC, unknown message types, and zero terminal dimensions must return errors without panicking; pinned by Task 1 parser tests.
- Arbitrary bytes split across PTY reads must reach xterm.js unchanged and in order; pinned by Task 1 binary transport tests and Task 2 ordered-event test.
- Output arriving before webview readiness, including output immediately before exit, must be retained and delivered in order; pinned by Task 1 bridge tests.
- Input after exit must return an error and must not silently restart the shell; pinned by Task 2 lifecycle tests.

---

### Task 1: Terminal event protocol and buffering

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Create: `src/terminal_protocol.rs`
- Create: `docs/adr/0008-webview-terminal-and-portable-pty.md`
- Modify: `docs/open-questions.md`
- Test: unit tests in `src/terminal_protocol.rs`

**Interfaces:**
- Consumes: JSON strings from `window.ipc.postMessage` and raw PTY bytes from Task 2.
- Produces: `TerminalRequest`, `TerminalEvent`, `TerminalDelivery`, `TerminalBridge`, `parse_request(&str) -> Result<TerminalRequest, String>`, and `delivery_script(&TerminalDelivery) -> String`.

- [ ] **Step 1: Write parser tests before adding the module implementation**

Add tests requiring these exact accepted messages:

```text
{"type":"ready"}
{"type":"input","data":"printf 'hi'\r"}
{"type":"resize","cols":120,"rows":40}
```

Assert the resulting values are `Ready`, UTF-8 input bytes, and `Resize { cols: 120, rows: 40 }`. Add rejection tests for malformed JSON, an unknown `type`, unknown fields, `cols: 0`, `rows: 0`, and dimensions above `u16::MAX`.

- [ ] **Step 2: Run the new module test target and verify it fails to compile**

Run: `cargo test terminal_protocol::tests`

Expected: FAIL because `terminal_protocol` and its interfaces do not exist.

- [ ] **Step 3: Implement request parsing**

Add `serde_json = "1"` and define:

```rust
pub enum TerminalRequest {
    Ready,
    Input(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

pub fn parse_request(body: &str) -> Result<TerminalRequest, String>;
```

Use a private `#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]` wire enum. Reject zero dimensions explicitly and return descriptive errors rather than panicking.

- [ ] **Step 4: Run parser tests and verify they pass**

Run: `cargo test terminal_protocol::tests::parse_`

Expected: all parser tests PASS.

- [ ] **Step 5: Write failing bridge and binary-delivery tests**

Define tests that require:

- `TerminalBridge::push` retains output before `mark_ready` and `take_deliveries` returns nothing.
- `mark_ready` releases retained output.
- adjacent output events are coalesced without changing bytes, including `[0, 0xff, 0x1b, b'[', b'm']` split across events.
- `Exited` or `Failed` is delivered after all preceding output.
- restoring undelivered items after a simulated evaluation failure preserves their order.
- `delivery_script` base64-encodes output and safely JSON-quotes error text containing quotes and newlines.

- [ ] **Step 6: Implement event buffering and scripts**

Add `base64 = "0.23"` and define:

```rust
pub enum TerminalEvent {
    Output(Vec<u8>),
    Exited { code: Option<u32> },
    Failed(String),
}

pub enum TerminalDelivery {
    Output(Vec<u8>),
    Exited { code: Option<u32> },
    Failed(String),
}

#[derive(Default)]
pub struct TerminalBridge { /* private readiness and queue state */ }

impl TerminalBridge {
    pub fn push(&mut self, event: TerminalEvent);
    pub fn mark_ready(&mut self);
    pub fn take_deliveries(&mut self) -> Vec<TerminalDelivery>;
    pub fn restore_front(&mut self, deliveries: Vec<TerminalDelivery>);
}

pub fn delivery_script(delivery: &TerminalDelivery) -> String;
```

Generated scripts call only `window.rake.writeBase64(...)` or `window.rake.end(...)`. Use `serde_json` for JavaScript string literals and `base64::engine::general_purpose::STANDARD` for bytes.

- [ ] **Step 7: Run all protocol tests**

Run: `cargo test terminal_protocol::tests`

Expected: PASS, including malformed input, arbitrary bytes, pre-ready buffering, and exit ordering.

- [ ] **Step 8: Record the architecture decision**

ADR 0008 records bundled xterm.js 6.0.0 in `wry` 0.57/WKWebView plus `portable-pty` 0.9, the single-event-loop bridge, offline assets, and the consequences. Remove resolved renderer and emulator-core questions 1 and 2 from `docs/open-questions.md`; keep the remaining questions.

- [ ] **Step 9: Verify and commit**

Run separately: `cargo test`, `cargo clippy --all-targets`, `cargo fmt -- --check`.

Expected: all commands exit 0.

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/terminal_protocol.rs docs/adr/0008-webview-terminal-and-portable-pty.md docs/open-questions.md
git commit -m "feat: terminal protocol - buffer binary-safe webview events"
```

### Task 2: Persistent PTY session

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Create: `src/pty.rs`
- Test: unit and macOS integration tests in `src/pty.rs`

**Interfaces:**
- Consumes: `terminal_protocol::TerminalEvent` and a callback `Fn(TerminalEvent) + Send + 'static`.
- Produces: `ShellSpec`, `TerminalSize`, and `PtySession::{spawn, write, resize, is_running}`.

- [ ] **Step 1: Write failing shell-selection and size tests**

Require `default_login_shell(Some("/bin/bash".into()))` to produce program `/bin/bash` with argument `-l`; require `None` and an empty value to produce `/bin/zsh` with `-l`. Require `TerminalSize::new(0, 24)` and `new(80, 0)` to fail and `new(80, 24)` to succeed.

- [ ] **Step 2: Run the focused tests and verify they fail to compile**

Run: `cargo test pty::tests::default_ pty::tests::terminal_size_`

Expected: FAIL because `pty` does not exist; run filters separately if needed.

- [ ] **Step 3: Implement shell and size value types**

Define:

```rust
pub struct ShellSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

pub fn default_login_shell(shell: Option<OsString>) -> ShellSpec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalSize { pub cols: u16, pub rows: u16 }

impl TerminalSize {
    pub fn new(cols: u16, rows: u16) -> Result<Self, String>;
}
```

- [ ] **Step 4: Run shell and size tests and verify they pass**

Run the two test filters from Step 2 separately.

Expected: PASS.

- [ ] **Step 5: Write failing PTY lifecycle tests**

Using `/bin/sh -c "printf rake-pty-ok"`, assert `PtySession::spawn` emits `Output` containing those exact bytes followed by `Exited`, never the reverse. Add a test that a completed session reports `is_running() == false`, `write` after completion returns an error, and dropping a running `/bin/cat` session does not hang.

- [ ] **Step 6: Implement `PtySession` with ordered output and exit**

Add `portable-pty = "0.9"` and define:

```rust
pub struct PtySession { /* master, writer, child killer, running state, worker */ }

impl PtySession {
    pub fn spawn<F>(spec: ShellSpec, size: TerminalSize, notify: F) -> Result<Self, String>
    where F: Fn(TerminalEvent) + Send + 'static;
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), String>;
    pub fn resize(&mut self, size: TerminalSize) -> Result<(), String>;
    pub fn is_running(&self) -> bool;
}
```

Open the PTY with the requested size, set `TERM=xterm-256color`, spawn the command, and move both reader and child into one worker so it reads to EOF before waiting and emitting `Exited`. On a read failure, terminate the child and emit `Failed`; on a wait failure emit `Failed`. Keep a child killer for `Drop`; do not join indefinitely during GUI shutdown.

- [ ] **Step 7: Run PTY tests**

Run: `cargo test pty::tests`

Expected: PASS with output-before-exit ordering and no hanging test.

- [ ] **Step 8: Verify and commit**

Run separately: `cargo test`, `cargo clippy --all-targets`, `cargo fmt -- --check`.

Expected: all exit 0.

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/pty.rs
git commit -m "feat: PTY session - run a persistent login shell"
```

### Task 3: Bundled xterm.js terminal view

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Create: `src/terminal_view.rs`
- Create: `assets/terminal/index.html`
- Create: `assets/terminal/rake.js`
- Create: `assets/terminal/xterm.css`
- Create: `assets/terminal/xterm.js`
- Create: `assets/terminal/addon-fit.js`
- Create: `assets/licenses/xterm-LICENSE`
- Create: `assets/licenses/addon-fit-LICENSE`
- Modify: `src/dropdown.rs`
- Test: unit tests in `src/terminal_view.rs`

**Interfaces:**
- Consumes: raw IPC body callback and Task 1 `TerminalBridge`/`TerminalEvent`.
- Produces: `terminal_html() -> String` and `TerminalView::{new, mark_ready, queue, flush, focus, set_bounds}`; `DropDown` delegates terminal-view operations.

- [ ] **Step 1: Write failing offline-document tests**

Require `terminal_html()` to contain xterm.js 6.0.0, addon-fit 0.11.0, the `window.rake` bridge, a full-window terminal container, and no remote `<script src="http...">` or `<link href="http...">`. Require the app script to post `ready`, UTF-8 `input`, and positive-integer `resize` messages and expose `writeBase64`, `end`, and `focus`.

- [ ] **Step 2: Run the view tests and verify they fail to compile**

Run: `cargo test terminal_view::tests`

Expected: FAIL because the module and assets do not exist.

- [ ] **Step 3: Vendor pinned xterm assets and licenses**

Copy only `lib/xterm.js`, `css/xterm.css`, and `LICENSE` from `@xterm/xterm@6.0.0`; copy `lib/addon-fit.js` and `LICENSE` from `@xterm/addon-fit@0.11.0`. Do not add npm, `node_modules`, source maps, CDN references, or a JavaScript build pipeline.

- [ ] **Step 4: Build the local terminal document**

Create the HTML/CSS/app script with a dark full-window terminal, system monospace font, xterm `Terminal`, and `FitAddon`. Use `ResizeObserver` to call `fit()` and post changed dimensions. Use xterm's `onData` for input. Handle `Cmd+C` as copy when a selection exists and `Cmd+V` as paste while leaving `Ctrl+C` to xterm. Decode output base64 into `Uint8Array` before `terminal.write`.

Implement `pub fn terminal_html() -> String` by embedding the local assets with `include_str!`; do not read assets from disk at runtime.

- [ ] **Step 5: Run document tests and verify they pass**

Run: `cargo test terminal_view::tests`

Expected: PASS.

- [ ] **Step 6: Write failing `TerminalView` bridge tests around a pure delivery helper**

Keep webview calls at the edge. Add tests to a private/pure helper showing that `queue` before readiness evaluates nothing, `mark_ready` then `flush` returns scripts in order, adjacent output is one delivery, and a second flush with no events is empty.

- [ ] **Step 7: Implement the webview-backed view and replace softbuffer**

Add `wry = { version = "0.57", default-features = false }` on macOS and implement:

```rust
pub struct TerminalView { /* WebView plus TerminalBridge */ }

impl TerminalView {
    pub fn new<F>(window: &tao::window::Window, ipc: F) -> Result<Self, String>
    where F: Fn(String) + 'static;
    pub fn mark_ready(&mut self);
    pub fn queue(&mut self, event: TerminalEvent);
    pub fn flush(&mut self) -> Result<(), String>;
    pub fn focus(&self) -> Result<(), String>;
    pub fn set_bounds(&self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<(), String>;
}
```

Build with `WebViewBuilder::with_html`, `with_ipc_handler`, navigation denied, and new-window requests denied. Convert the physical size to logical bounds with the supplied scale factor. `flush` evaluates Task 1 scripts on the GUI thread; if an evaluation fails, restore the failed and remaining deliveries to the front of the bridge before returning the error. Change `DropDown::new` to accept the IPC callback, own `TerminalView`, remove `softbuffer::Surface`, and remove placeholder paint/redraw code. Forward view methods through narrowly named `DropDown` methods.

- [ ] **Step 8: Run all automated checks**

Run separately: `cargo test`, `cargo clippy --all-targets`, `cargo fmt -- --check`.

Expected: all exit 0; no `softbuffer` dependency remains.

- [ ] **Step 9: Manually verify the local terminal surface**

Run: `cargo run`

Expected: Rake opens a dark xterm surface with no network access or navigation. It has no shell output yet. Resizing/initial sizing fills the panel without uncovered placeholder pixels.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock src assets
git commit -m "feat: terminal view - embed xterm.js in the drop-down"
```

### Task 4: Wire the shell, webview, and launch behavior

**Files:**
- Modify: `src/main.rs`
- Modify: `src/dropdown.rs`
- Modify: `src/terminal_view.rs`
- Modify: `README.md`
- Modify: `docs/design.md`
- Modify: `docs/roadmap.md`
- Test: existing unit tests plus Task 1–3 tests

**Interfaces:**
- Consumes: `TerminalRequest`, `TerminalEvent`, `parse_request`, `PtySession`, `ShellSpec`, `TerminalSize`, and `TerminalView` from Tasks 1–3.
- Produces: one functioning persistent terminal connected through `AppEvent`, shown and focused on direct launch.

- [ ] **Step 1: Add the application events and preserve thin wiring**

Extend `AppEvent` with `TerminalRequest(String)` and `Terminal(TerminalEvent)`. Pass an IPC closure to `DropDown::new` that forwards the raw body through `EventLoopProxy`. Create `PtySession` after the view with `default_login_shell(std::env::var_os("SHELL"))`, `TerminalSize::new(80, 24)`, and a callback that forwards `TerminalEvent` through the same proxy.

- [ ] **Step 2: Route webview requests without panicking**

For `Ready`, mark the view ready and request terminal focus. For `Input`, call `PtySession::write` only while running. For `Resize`, validate through `TerminalSize::new` and call `resize`. Malformed messages are reported to stderr and ignored; PTY operation failures queue `TerminalEvent::Failed` rather than panic.

- [ ] **Step 3: Route PTY events and flush on the GUI thread**

Queue every `TerminalEvent` in `DropDown`. On `MainEventsCleared`, tick animation and call terminal flush. When `Exited` or `Failed` arrives, mark application-side session state ended so later input is ignored. Preserve event order.

- [ ] **Step 4: Make direct launch shown and focused**

Initialize visibility as `Shown`, call `dropdown.show(Instant::now())` after creating the drop-down and PTY, and have `DropDown::show` restore xterm focus whenever the panel is shown. Do not add hidden-start or login-item behavior in this milestone.

- [ ] **Step 5: Remove obsolete paint event handling**

Remove `RedrawRequested`, placeholder painting, and softbuffer-specific redraw paths. Keep resize handling for webview bounds and PTY size notifications from JavaScript.

- [ ] **Step 6: Run automated verification**

Run: `cargo test`

Expected: all tests pass, including protocol arbitrary-byte/order tests and PTY output-before-exit tests.

Run separately: `cargo clippy --all-targets`, `cargo fmt -- --check`, `git diff --check`.

Expected: all exit 0 and `git diff --check` has no output.

- [ ] **Step 7: Run the macOS terminal checklist**

Run `cargo run` and verify all of the following:

1. Rake opens immediately and keyboard focus is in the shell.
2. The shell is the `$SHELL` login shell; `echo $TERM` reports `xterm-256color`.
3. `pwd`, `cd`, ANSI colors, cursor movement, and a full-screen interactive program work.
4. `Ctrl+C` interrupts a foreground command.
5. Selection plus `Cmd+C`, and `Cmd+V`, work as macOS terminal shortcuts.
6. `seq 1 100000` completes without missing/reordered output or freezing the UI indefinitely.
7. Hiding and showing preserves `pwd`, running jobs, screen, and scrollback.
8. Window sizing produces sensible rows/columns and interactive programs respond to resize.
9. Typing `exit` leaves the final screen visible with a process-ended message and does not restart a shell.
10. The panel still slides under the menu bar, rests below it, and the known startup hitch is recorded if still visible.

- [ ] **Step 8: Update honest current-state documentation**

Update README status/run behavior, `docs/design.md`, and `docs/roadmap.md` to say the single terminal session exists. Keep tabs, profiles, Settings, login launch, polish, and cross-platform explicitly planned rather than implemented.

- [ ] **Step 9: Commit the integrated terminal**

```bash
git add src README.md docs/design.md docs/roadmap.md
git commit -m "feat: terminal session - connect the login shell to xterm.js"
```

- [ ] **Step 10: Final whole-plan verification**

Run separately: `cargo test`, `cargo clippy --all-targets`, `cargo fmt -- --check`, `git status --short`.

Expected: tests pass, clippy and formatting are clean, and status contains only intentional planning/spec files if they were not committed before execution.
