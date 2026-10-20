# 4. Testable library with a thin `main.rs`

Date: 2026-10-19 · Status: Accepted

## Context
The project follows test-driven development, but GUI event loops and OS calls are hard to unit test.

## Decision
Put pure logic in `src/lib.rs` and later modules, where it is unit tested. Keep `src/main.rs` as thin wiring around the event loop. Verify event-loop behavior by hand and record how.

## Consequences
Most logic is covered by fast tests. The glue in `main.rs` has no automated coverage, so it must stay small, and it is the first place to look when something breaks that the tests didn't catch.
