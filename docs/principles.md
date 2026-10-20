# Principles

## Test-driven development

Every capability starts with a failing test.

1. Write a failing test that defines the behavior.
2. Write the least code that makes it pass.
3. Refactor with the tests green.
4. Commit.

Code that is hard to test (event-loop glue, OS calls) is kept thin and pushed to the edges. If a piece of logic can only be tested through a GUI, look for a pure core to extract first. Behavior that can't be automated is verified by hand and written down in the commit or PR.

## Architecture decision records

A significant choice, such as a crate, a platform trade-off or a structural change, gets a short record in [`docs/adr/`](adr/): context, decision, consequences. Records are never rewritten. A changed decision gets a new record that supersedes the old one.

## Design principles

- **Smallest thing that works.** Build only what the current milestone needs. No scaffolding for features that don't exist yet.
- **No dead code.** Stubs, placeholder modules and unused dependencies don't get committed. If it isn't used, delete it. Git remembers.
- **One event loop.** See [design](design.md).
- **Platform code is isolated.** `cfg(target_os)` code lives in small, clearly named functions.
- **Fail loudly at startup, never silently.** Unrecoverable setup errors panic with a clear message. Recoverable runtime errors are handled, not ignored.
- **Honest docs.** Documentation describes what exists. Plans live in the [roadmap](roadmap.md) and say they are plans.

## Programming conventions

- Stable Rust, 2021 edition.
- `cargo fmt` and `cargo clippy --all-targets` clean before every commit.
- Prefer small functions and types with explicit inputs over globals.
- Comments explain *why*, not *what*.
- Add a dependency only when it removes real work, and note it in an ADR if the choice is significant.

## Commits

One logical change per commit, with the tests that cover it. Use a conventional prefix: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`. The body explains why when the reason isn't obvious.
