# quake-terminal

[![CI](https://github.com/jdtobe/rake/actions/workflows/ci.yml/badge.svg)](https://github.com/jdtobe/rake/actions/workflows/ci.yml)

A Quake-style drop-down terminal, written in Rust. Press a hotkey and a terminal slides down from the top of the screen; press it again and it slides away.

## Status

**Early. Only the first step exists.**

| Capability | State |
|---|---|
| Menubar icon with **About** and **Quit** | Working (macOS) |
| Global hotkey | Working |
| Drop-down window (top of screen, dark placeholder) | Working |
| Slide animation | Working; short hitch at the start of each slide in |
| Configuration | Hotkey, window size, hide on focus lost, animation time (config file only) |
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

# Slide in/out time in milliseconds, 0 to 2000. 0 turns the animation off. Default: 300
animation_ms = 300
```

Hotkey strings use the [`global-hotkey`](https://docs.rs/global-hotkey) format, such as `Ctrl+Shift+K`, `Cmd+Backquote` or `F12`. Note that a global hotkey is captured system-wide, so the default `Ctrl+\`` will also stop working inside other apps that use it, such as editors.

## Develop

```bash
cargo test      # unit tests
cargo clippy --all-targets
cargo fmt
```

Requires a stable Rust toolchain.

Every push runs formatting, lints and tests on macOS in CI; pull requests run the same
checks across all targets and the doc tests. Releases are cut by hand from the Actions
tab, never on merge, and are tagged `v{YY}.{month}.{build}` (the first release of
October 2026 is `v26.10.0`).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security problems should follow
[SECURITY.md](SECURITY.md) rather than be filed as public issues.

## Licence

[MIT](LICENSE).

## Documentation

- [Roadmap](docs/roadmap.md): goals, planned feature set and milestones
- [Design](docs/design.md): architecture and complications
- [Principles](docs/principles.md): how we work (TDD, ADRs, commits)
- [Open questions](docs/open-questions.md): decisions not yet made
- [Architecture decision records](docs/adr/): why things are the way they are
- [AGENTS.md](AGENTS.md): instructions for AI coding agents
