# Rake Terminal Design

Date: 2026-10-22

## Intent

Rake is a macOS-first, Quake-style drop-down terminal. It should feel familiar to someone who normally uses macOS Terminal: launch a real login shell, accept ordinary terminal input, preserve the session while hidden, and appear immediately on demand.

The current application already proves the menubar, global hotkey, drop-down window, configuration, and slide animation. This design renames the remaining `quake-terminal` identifiers to Rake and replaces the dark placeholder with the first usable terminal session.

## Product decisions

- The product name is **Rake**. The Cargo package, executable, configuration directory, documentation, window title, and displayed metadata use that brand.
- The terminal is a webview running xterm.js. xterm.js supplies both terminal emulation and rendering; Rake does not add a separate Rust terminal-emulator core.
- Rust owns the shell process and pseudoterminal (PTY).
- The first release has one terminal session using the user's default macOS login shell.
- Hiding Rake affects only window visibility. The webview, shell, child processes, current directory, and scrollback remain alive.
- A direct launch creates the shell immediately, opens the drop-down, and focuses it.
- A future “Launch Rake at login” setting will start the same application hidden. That setting and login-item integration are not part of this milestone.
- If the shell exits, the terminal remains open and displays that the process ended. It does not automatically restart.
- Tabs are an intended later feature. Profiles are also intended, but their contents will be designed only after Rake has enough real settings to group meaningfully.

## Scope

### Included

- Complete rename from `quake-terminal` to Rake.
- One default login-shell session, started when Rake launches.
- Bidirectional terminal I/O.
- ANSI colors, cursor behavior, and terminal semantics supplied by xterm.js.
- Terminal resizing when the webview's fitted row and column count changes.
- Scrollback and text selection.
- macOS-style copy and paste without changing `Ctrl+C` terminal behavior.
- Existing hotkey, focus, geometry, animation, and config behavior.
- A visible process-ended state after shell exit.
- Automated tests for pure state and configuration logic, with documented manual checks for PTY/webview/GUI integration.

### Deferred

- Tabs and tab UI.
- Profile definition and profile UI.
- In-app Settings UI.
- Launch-at-login registration and hidden login launch.
- Shell selection, font, theme, opacity, and scrollback settings.
- Full-screen Spaces and final multi-monitor behavior.
- Linux and Windows support.
- Packaging, signing, and notarization as a `.app` bundle.

## Rename

The rename is complete rather than cosmetic:

- Cargo package and binary: `rake`
- Rust library crate: `rake`
- App/window/About name: `Rake`
- Config path: `~/.config/rake/config.toml`
- Documentation and source comments: `Rake`

The old configuration path is not retained or migrated. Rake is still an early prototype, and preserving the previous name in compatibility code would work against the explicit branding decision. Existing local configuration can be moved manually.

## Architecture

### Existing event loop

Rake keeps its single `tao` event loop. The event loop continues to own all GUI state, including the tray, drop-down window, and webview. PTY work may run on background threads, but those threads communicate with the event loop rather than mutating GUI state directly.

### Webview terminal

The drop-down window hosts a `wry` webview backed by WKWebView on macOS. Its HTML, JavaScript, xterm.js code, and styles are packaged with Rake and loaded locally; normal terminal operation does not depend on a network connection. Vendored web assets retain their required license notices.

The page creates one xterm.js `Terminal` and a fit addon. JavaScript reports readiness, terminal input, and size changes to Rust through the webview IPC handler. Rust forwards terminal output and process-exit notifications to JavaScript on the GUI thread. Output that arrives before the page reports readiness is retained and delivered in order afterward.

The page is terminal content, not a general browser. Rake does not expose unrestricted navigation or remote web content.

### PTY session

A session uses `portable-pty` and owns:

- the PTY master and child process,
- a writer for input,
- a reader thread for output,
- current row and column dimensions,
- lifecycle state: running or exited.

The shell comes from `$SHELL`, with `/bin/zsh` as the macOS fallback, and is invoked as a login shell. The PTY advertises a terminal type compatible with xterm.js, such as `xterm-256color`.

The reader thread sends output chunks into the `tao` event loop. The GUI side queues them, coalesces adjacent chunks to avoid one JavaScript evaluation per small read, and delivers all bytes to xterm.js in order once the page is ready. Input received from xterm.js is written to the PTY. Resize messages update the PTY dimensions. Output is never intentionally dropped; queue growth and event-loop responsiveness are checked under high-output commands during integration testing.

Output must be transported as bytes without assuming UTF-8 chunk boundaries. The bridge therefore uses a binary-safe representation rather than inserting raw output into generated JavaScript strings.

### Session lifetime

The session starts during application initialization, before the initial window is shown. Showing and hiding never creates or destroys a session.

When the shell exits, the session records the exit state and tells xterm.js to append a process-ended message. Further terminal input is ignored. Rake itself and its menu remain running.

Although the first version has one session, PTY ownership and lifecycle live in a dedicated session type rather than in `main.rs`. A later tab can own one instance of that same type. No tab manager or generic profile abstraction is introduced yet.

### Window and webview ownership

The current `DropDown` remains responsible for visibility, placement, focus, and animation. Its placeholder `softbuffer` surface is replaced by a webview attached to the same window. The existing pure geometry and animation modules remain unchanged unless webview integration reveals a concrete need.

The webview remains attached and alive while the window is hidden, preserving xterm.js screen and scrollback state. Each time the panel is shown, Rake explicitly restores focus to the xterm.js terminal after making the window active.

## Interaction behavior

### Startup

1. Rake loads and validates configuration.
2. It creates the event loop, tray, drop-down window, and terminal webview.
3. It starts the default login shell in a PTY.
4. It shows the drop-down and focuses the terminal.

The existing hotkey subsequently toggles visibility as before.

### Keyboard and clipboard

- Ordinary key input is delivered to xterm.js and then to the PTY.
- `Ctrl+C` remains terminal input and can interrupt the foreground process.
- Standard macOS copy and paste use `Cmd+C` and `Cmd+V`.
- Copy uses the current terminal selection; paste sends clipboard text as terminal input.
- Text selection and scrollback remain available while the shell is running or after it exits.

### Resize

The xterm.js fit addon calculates rows and columns from the webview size. It reports changes to Rust, which resizes the PTY. The shell receives the resulting terminal-size change through normal PTY behavior.

### Shell exit

When the shell process exits, Rake appends a short process-ended message that may include the exit status when available. It keeps the completed screen and scrollback visible and does not silently create a replacement shell.

## Error handling

- Failure to create the initial window, webview, PTY, or shell is a startup failure with a clear error message.
- Malformed webview IPC messages are rejected without panicking the event loop.
- Runtime PTY read/write failures transition the session to an ended/error state and report that state in the terminal.
- Configuration continues to fail loudly at startup as it does today.

## Testing and verification

Pure logic remains outside GUI glue and is developed test-first. Automated coverage includes:

- Rake config-path and package metadata changes.
- Shell-command selection and login-shell arguments.
- Session lifecycle transitions.
- Input, output, resize, and exit message decoding.
- Binary-safe output transport across arbitrary byte boundaries.
- Rejection of malformed IPC messages.

Integration behavior requiring macOS GUI or PTY facilities is verified manually:

- Direct launch opens and focuses a working shell.
- Commands, interactive programs, colors, cursor movement, and `Ctrl+C` work.
- `Cmd+C` and `Cmd+V` behave like macOS terminal shortcuts.
- Resizing produces correct rows and columns.
- Hiding and showing preserves processes, directory, screen, and scrollback.
- Exiting the shell leaves the completed terminal visible.
- The panel still slides under the menu bar and rests below it.

Before each logical commit, `cargo test`, `cargo clippy --all-targets`, and `cargo fmt -- --check` must pass.

## Delivery sequence

1. Rename the project and configuration path to Rake, updating tests and documentation.
2. Record the accepted webview/xterm.js and PTY architecture in an ADR.
3. Add a testable PTY session and event protocol.
4. Replace the placeholder renderer with the local xterm.js webview.
5. Connect output, input, resize, focus, clipboard, and process-exit behavior.
6. Change direct startup from hidden to shown and focused.
7. Run automated checks and the macOS manual-verification checklist.

Each step is a separate, coherent change with its own tests and commit. Tabs, profiles, Settings, and launch-at-login begin only after this single-session milestone is usable.
