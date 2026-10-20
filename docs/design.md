# Design

## Current architecture

```
src/main.rs     Event loop and wiring (thin, not unit tested)
src/lib.rs      Pure, testable helpers (icon pixels, About metadata)
src/hotkey.rs   Hotkey parsing and the show/hide toggle state
src/config.rs   TOML config loading and validation
src/geometry.rs Where the drop-down window goes (pure, tested)
src/animation.rs Easing and slide maths (pure, tested)
src/dropdown.rs The window itself: tao + softbuffer glue (checked by hand)
```

The global hotkey handler runs on another thread. It forwards each event into the loop as a `tao` user event (`AppEvent::Hotkey`), so all state changes happen on the main thread.

While a slide runs, the loop spins with `ControlFlow::Poll`; otherwise it sleeps ([ADR 6](adr/0006-animation-on-the-event-loop.md)). The window sits at the macOS menubar's own level so it is not clamped below the menubar and can slide in from off-screen, passing under the menubar, and rests just below it.

`main.rs` builds a [`tao`](https://crates.io/crates/tao) event loop, sets the macOS activation policy to `Accessory` (no Dock icon), and creates a [`tray-icon`](https://crates.io/crates/tray-icon) with a menu of `About` (native panel via predefined menu item) and `Quit`. The tray icon is created on `StartCause::Init` and kept alive for the lifetime of the loop, as macOS requires.

## Design intent

**One event loop owns everything.** The tray, the global hotkey and the window all hang off the single `tao` event loop. New capabilities add event handlers; they do not add threads or a second loop, which macOS would not allow on the main thread anyway.

**Keep logic out of the event loop.** Anything that can be a pure function or a small type with plain inputs and outputs goes in `lib.rs` (and later its own modules), where it can be unit tested without a GUI. `main.rs` stays glue.

**Planned shape** (not yet built, subject to change):

| Concern | Candidate |
|---|---|
| Event loop, window | `tao` |
| Tray / menu | `tray-icon` |
| Global hotkey | `global-hotkey` (same family as `tao`) |
| PTY | `portable-pty` |
| Terminal emulation | `alacritty_terminal` or `vte` |
| Rendering | Webview with xterm.js (`wry`), or GPU text rendering. See [open questions](open-questions.md) |
| macOS-specific window behavior | small `objc2` calls, behind `cfg(target_os = "macos")` |

Platform-specific code stays behind `cfg` gates and small functions so a second platform adds code rather than rewriting the first.

## Complications we expect

- **Main-thread GUI.** macOS requires UI work on the main thread. Terminal I/O (PTY reads) must run on other threads and send events to the loop.
- **Menubar apps and focus.** An `Accessory` app can have trouble taking keyboard focus for a window. A drop-down terminal must grab focus reliably when shown.
- **Full-screen apps and Spaces.** Showing over a full-screen app needs specific window level and collection-behavior settings that `tao` may not expose, which would need `objc2`.
- **Global hotkey permissions.** Whether a global hotkey needs any macOS permission prompt needs checking on the target OS version.
- **Terminal rendering is the big unknown.** Emulating a terminal correctly (escape sequences, wide characters, IME, performance) is much larger than the rest of the app combined.
- **Running from `cargo run` vs a bundle.** A bare binary has no `Info.plist` or bundle identity. Some macOS features, such as notifications and a custom About icon, may need a real `.app` bundle later.
- **Cross-platform.** Wayland and X11 differ on global hotkeys and on positioning borderless windows, so Linux may be a poor fit for some of this.
