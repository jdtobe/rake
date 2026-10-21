# Handoff

Temporary note for the next session. Delete it once read.

## State
- Branch `main`, clean tree at `4dad0b2` (fix: slide under the menubar). 55 tests, clippy and fmt clean.
- Done: menubar icon, global hotkey, TOML config, drop-down window, width/height/hide-on-focus settings, slide animation (ADR 6, amendments 1-3).
- The window uses level 24 (`NSMainMenuWindowLevel`): unclamped, and drawn under the menubar. Level 23 is clamped, 25 draws over the menubar.

## First thing to do after restart
Screen & Audio Recording permission was just granted to the terminal, so `screencapture` should work now. Use it to verify the slide visually instead of relying on the window list:
1. Set `animation_ms = 2000` in a config under a scratch `HOME` (`$HOME/.config/quake-terminal/config.toml`), run `cargo build`, then run `target/debug/quake-terminal` with that `HOME`.
2. Trigger a show and capture mid-slide, for example `screencapture -x -R0,0,900,200 out.png`, and read the PNG.
3. Confirm the panel passes under the menubar and rests just below it.

There is no built-in way to trigger a toggle from outside. The earlier approach was a temporary thread sending an event through the `EventLoopProxy`. Never commit that instrumentation.

## Known issues
- A 40-70 ms hitch early in each slide-in (window ordered in and resized). A display-synchronised callback (`CVDisplayLink`) is the next step if it is noticeable.
- The window can appear on a different monitor depending on where macOS first placed it. Multi-monitor handling is deferred to the polish milestone.
- The user has not yet confirmed the under-the-menubar fix by eye.

## Next work (needs the user's go-ahead)
1. In-app Settings menu from the menubar item (hotkey, width/height, hide-on-focus). See `docs/roadmap.md`.
2. The terminal itself. Blocked on open questions 1 and 2 (renderer, emulator core) in `docs/open-questions.md`.
3. Polish: full-screen Spaces, multi-monitor, focus. Then cross-platform.

## Project rules
See `AGENTS.md`: TDD, one logical change per commit with a `feat:`/`fix:`/`docs:` prefix, no dead code, honest docs, ADRs are never edited except by dated amendments.
