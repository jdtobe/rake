# Rake

[![CI](https://github.com/jdtobe/rake/actions/workflows/ci.yml/badge.svg)](https://github.com/jdtobe/rake/actions/workflows/ci.yml)

Rake is a macOS-first Quake-style drop-down terminal written in Rust. Press a hotkey and a terminal panel slides down from the top of the screen; press it again and it slides away.

## Status

**Early. The first single-session terminal works on macOS; settings and polish are next.**

| Capability | State |
|---|---|
| Menubar icon with **About** and **Quit** | Working (macOS); template mark, tinted by the system |
| Left-click on the menubar icon | Toggles the panel; right-click opens the menu |
| **About** panel | Native panel with the colour app icon, version, copyright and credits |
| One running copy | A second launch brings the existing panel forward and exits |
| Global hotkey | Working |
| Drop-down window at the top of the screen | Working; opens on the display under the mouse and shows on every Space |
| Slide animation | Working; short hitch at the start of each slide in |
| Configuration | Hotkey, window size, hide on focus lost, animation time (config file only) |
| Settings menu in the menubar | Not started |
| Embedded terminal (shell + rendering) | Working as one persistent login-shell session; WebGL with a DOM fallback |
| Terminal colours and font | Follows the macOS Terminal default profile; changes apply on restart |
| Tabs and profiles | Planned after the single-session terminal is validated |
| Linux / Windows | Not tested |

Developed and tested on macOS only.

## Install

Download the latest `Rake-v….dmg`, open it and drag **Rake** onto **Applications**. The build is ad-hoc signed and not notarised, so the first launch needs Finder: right-click **Rake** and choose **Open** (macOS remembers the choice). Releases are manual; there is no auto-update.

## Run it

```bash
cargo run
```

Rake opens and focuses a terminal backed by your default login shell. The panel appears on the display under the mouse and stays there until hidden; the next show chooses again. The shell remains alive when the hotkey hides the window. An icon also appears in the menubar (no Dock icon): left-click toggles the panel, right-click opens **About** and **Quit**.

Only one copy runs. Launching Rake again, from Finder or the binary, brings the existing panel forward and exits the new process, so quit the running copy before `cargo run`.

## Configure

Optional. Create `~/.config/rake/config.toml`; a missing file means defaults. Changes need a restart. A bad file stops the app with an error naming the file and the problem.

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

The panel opens at the configured size and cannot be resized by dragging.

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
October 2026 is `v26.10.0`). A release publishes `Rake-v….dmg`, built by
`scripts/package.sh`; run that script after `cargo build --release` to make the same
artefact locally.

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
