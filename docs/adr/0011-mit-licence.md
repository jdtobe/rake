# 11. Licence Rake under MIT

Date: 2026-10-26 · Status: Accepted

## Context
Rake is published on GitHub as a macOS-first terminal. It vendors xterm.js 6.0.0, `@xterm/addon-fit` 0.11.0 and `@xterm/addon-webgl` 0.19.0, all MIT, and depends on crates under MIT or Apache-2.0. The licence has to sit comfortably with those and let people use, modify and share the app without friction.

## Decision
- Rake is licensed under MIT: `LICENSE`, `license = "MIT"` in `Cargo.toml`, and the README.
- Vendored assets keep their own licence files in `assets/licenses/`.
- Contributions are accepted under the same licence, stated in `CONTRIBUTING.md`.

## Consequences
Anyone may use, modify and redistribute Rake, including in closed-source work, provided the copyright and permission notices travel with it. There is no copyleft requirement. The MIT grant covers Rake's own code; bundled components remain under their own notices.
