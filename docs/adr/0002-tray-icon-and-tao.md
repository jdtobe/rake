# 2. Use `tray-icon` and `tao` for the menubar and event loop

Date: 2026-10-19 · Status: Accepted

## Context
The first milestone is a menubar icon with About and Quit, started with a plain `cargo run`. The later features are a global hotkey, an animated drop-down window and a terminal, and we want cross-platform to stay possible. The options were `tray-icon` with `tao`, or direct AppKit calls through `objc2`.

## Decision
Use `tray-icon` for the icon and menu, and `tao` for the event loop and, later, windows. Use the predefined About and Quit menu items, which are native on macOS.

## Consequences
- `tray-icon`, `tao` and `global-hotkey` come from the same project and are designed to share one event loop.
- Linux and Windows remain possible with the same crates.
- Some macOS-specific window behavior may still need small `objc2` calls (see [design](../design.md)).
- We depend on a third-party GUI stack instead of writing AppKit code directly.
- Rejected: raw `objc2`/AppKit. It gives the most macOS control, but it is macOS-only and needs a hand-written event loop and more unsafe code.
