# Roadmap

## Goal

A fast, native, always-available terminal that drops down from the top of the screen on a global hotkey, in the spirit of the console in Quake. macOS first, cross-platform if practical.

## Expected feature set

**Core**
- Menubar/tray icon with About and Quit
- Global hotkey toggles the terminal
- Borderless, always-on-top window that slides in and out
- A real shell session in a PTY, with a proper terminal emulator (colors, cursor, resize, scrollback)

**Later**
- Configuration file (hotkey, height, shell, position, opacity)
- **Settings menu** in the menubar icon's context menu, so settings such as the hotkey, window width and height, and hide-on-focus-lost can be changed in the app instead of by editing the config file. The config file stays the source of truth; the Settings UI reads and writes it.
- Show over full-screen apps and on every Space
- Appear on the display that holds the mouse or focused window
- Tabs or splits (only if wanted; not assumed)
- Linux and Windows support

**Non-goals for now**
- Replacing a full terminal application
- Plugins or scripting

## Milestones

Each milestone is a small, shippable slice built test-first (see [principles](principles.md)).

1. **Menubar icon (done):** `cargo run` shows an icon with About and Quit.
2. **Hotkey (done):** a global hotkey fires an event in the existing event loop and toggles the visibility state. The default is `Ctrl+\``, and it is user-configurable via the TOML config file ([ADR 5](adr/0005-toml-config-read-at-startup.md)).
3. **Window (done):** a borderless, always-on-top window appears at the top of the screen, centered, and disappears on the hotkey, filled with a dark placeholder. Size is configurable (`width_percent`, `height_percent`), as is hiding on focus loss (`hide_on_focus_lost`).
4. **Animation (done, pending manual check):** the window slides in from above the top edge and back out instead of popping. Toggling mid-slide reverses smoothly. Configurable with `animation_ms` ([ADR 6](adr/0006-animation-on-the-event-loop.md)).
5. **Terminal:** shell in a PTY, rendered in the window.
6. **Config:** user-editable settings with sensible defaults. The hotkey is done via the config file; the rest of the settings and the in-app Settings menu follow.
7. **Polish:** full-screen Spaces, multi-monitor, focus behavior.
8. **Cross-platform:** Linux and Windows.

Milestones 1 to 4 are the core drop-down behavior. Next is the terminal (5), with polish and the in-app Settings menu after it.
