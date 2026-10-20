# Open questions

Decisions we haven't made yet. Resolve one by recording an [ADR](adr/) and removing it from here.

1. **How do we render the terminal?** A webview running xterm.js is the least work, but adds a web runtime and some latency. GPU text rendering (for example `wgpu` plus `alacritty_terminal`) is faster and more native, but far more work.
2. **Which emulator core?** `alacritty_terminal`, `vte` plus our own grid, or something else.
3. **Milestone order.** Should a minimal terminal come before the slide animation, so the animation can be judged on real content?
4. **Does `tao` expose enough window control on macOS?** In particular: window level, not stealing focus, and showing over full-screen apps. If not, do we patch via `objc2`, or move to a native `NSPanel`?
5. **Default hotkey.** The earlier draft used `Ctrl+\``. Does that conflict with common shortcuts or with shells and editors?
6. **Configuration format and location.** TOML at the platform config dir is the likely answer. Hot reload or restart only?
7. **Distribution.** Is `cargo run` enough, or do we need a signed `.app` bundle (for notifications, a proper About icon, launch at login)?
8. **Menubar icon art.** The icon is currently a solid placeholder square.
9. **Linux and Windows.** Is cross-platform a real goal, or an option we keep open? This decides how much we invest in abstractions.
10. **Licensing and project name.** The crate is named `quake-terminal`, which may be too close to existing projects.
