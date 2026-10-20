# quake-terminal

A Quake-style drop-down terminal, written in Rust. Press a hotkey and a terminal slides down from the top of the screen; press it again and it slides away.

## Status

**Early. Only the first step exists.**

| Capability | State |
|---|---|
| Menubar icon with **About** and **Quit** | Working (macOS) |
| Global hotkey | Working |
| Drop-down window (top of screen, dark placeholder) | Working, appears and disappears instantly |
| Slide animation | Not started |
| Configuration | Hotkey, window size, hide on focus lost (config file only) |
| Settings menu in the menubar | Not started |
| Embedded terminal (shell + rendering) | Not started |
| Linux / Windows | Not tested |

Developed and tested on macOS only.

## Run it

```bash
cargo run
```

An icon appears in the menubar (no Dock icon). Click it for **About** (native panel) and **Quit**. `Ctrl+C` in the terminal also stops it.

## Configure

Optional. Create `~/.config/quake-terminal/config.toml`; a missing file means defaults. Changes need a restart. A bad file stops the app with an error naming the file and the problem.

```toml
# Global key that shows/hides the terminal. Default: "Ctrl+`"
hotkey = "Alt+Space"

# Window height as a percentage of the screen, 10 to 100. Default: 40
height_percent = 40

# Window width as a percentage of the screen, 10 to 100. The window is centered. Default: 100
width_percent = 100

# Hide the window when it loses keyboard focus. Default: false
hide_on_focus_lost = false
```

Hotkey strings use the [`global-hotkey`](https://docs.rs/global-hotkey) format, such as `Ctrl+Shift+K`, `Cmd+Backquote` or `F12`. Note that a global hotkey is captured system-wide, so the default `Ctrl+\`` will also stop working inside other apps that use it, such as editors.

## Develop

```bash
cargo test      # unit tests
cargo clippy --all-targets
cargo fmt
```

Requires a stable Rust toolchain.

## Documentation

- [Roadmap](docs/roadmap.md): goals, planned feature set and milestones
- [Design](docs/design.md): architecture and complications
- [Principles](docs/principles.md): how we work (TDD, ADRs, commits)
- [Open questions](docs/open-questions.md): decisions not yet made
- [Architecture decision records](docs/adr/): why things are the way they are
- [AGENTS.md](AGENTS.md): instructions for AI coding agents
