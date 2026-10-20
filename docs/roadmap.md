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
2. **Hotkey (done):** a global hotkey fires an event in the existing event loop and toggles a visibility state, logged to stderr. The default is `Ctrl+\``, and it is user-configurable via the TOML config file ([ADR 5](adr/0005-toml-config-read-at-startup.md)).
3. **Window:** a borderless window appears and disappears on the hotkey, with no terminal inside yet.
4. **Animation:** the window slides from the top edge instead of popping.
5. **Terminal:** shell in a PTY, rendered in the window.
6. **Config:** user-editable settings with sensible defaults.
7. **Polish:** full-screen Spaces, multi-monitor, focus behavior.
8. **Cross-platform:** Linux and Windows.

The order of 5 relative to 3 and 4 is an [open question](open-questions.md).
