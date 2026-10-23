# Open questions

Decisions we haven't made yet. Resolve one by recording an [ADR](adr/) and removing it from here.

1. **Does `tao` expose enough window control on macOS?** In particular: window level, not stealing focus, and showing over full-screen apps. If not, do we patch via `objc2`, or move to a native `NSPanel`?
2. **Distribution.** Is `cargo run` enough, or do we need a signed `.app` bundle (for notifications, a proper About icon, launch at login)?
3. **Menubar icon art.** The icon is currently a solid placeholder square.
4. **Linux and Windows.** Is cross-platform a real goal, or an option we keep open? This decides how much we invest in abstractions.
5. **Licensing.** Which license should Rake use?

Resolved: milestone order (window and animation come before the terminal, with a placeholder fill), default hotkey and config format, location and reload behavior ([ADR 5](adr/0005-toml-config-read-at-startup.md)), and terminal rendering/emulation ([ADR 8](adr/0008-webview-terminal-and-portable-pty.md)).
