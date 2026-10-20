# quake-terminal

A Quake-style drop-down terminal, written in Rust. Press a hotkey and a terminal slides down from the top of the screen; press it again and it slides away.

## Status

**Early. Only the first step exists.**

| Capability | State |
|---|---|
| Menubar icon with **About** and **Quit** | Working (macOS) |
| Global hotkey (`Ctrl+`` ) | Working, but fixed and only logs `Shown`/`Hidden` to stderr |
| Drop-down window with slide animation | Not started |
| Embedded terminal (shell + rendering) | Not started |
| Configuration | Not started |
| Linux / Windows | Not tested |

Developed and tested on macOS only.

## Run it

```bash
cargo run
```

An icon appears in the menubar (no Dock icon). Click it for **About** (native panel) and **Quit**. `Ctrl+C` in the terminal also stops it.

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
