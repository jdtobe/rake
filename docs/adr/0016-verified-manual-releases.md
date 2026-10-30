# 16. Shared checks and verified manual releases

Date: 2026-10-30 · Status: Accepted

## Context

Push and PR workflows duplicated macOS checks. Release builds had broad write
permissions, stamped only the manifest and did not inspect the packaged image.
Tabs add browser-side behaviour that needs failure evidence. Releases must remain
manual, with one drag-to-install DMG and calendar versioning.

## Decision

- PRs, main pushes and manual CI use one reusable checks workflow. Feature-branch
  pushes do not independently duplicate PR checks. macOS runs Rust/WebKit and
  real DMG tests; Ubuntu runs workflow/shell lint and portable script tests.
- Audit the full Cargo lockfile and npm development dependencies in shared checks
  and weekly. Fail on vulnerabilities; retain informational Rust warnings rather
  than silently suppressing them. Actions are SHA-pinned, analysis archives are
  checksum-pinned and cargo-audit is version-pinned. Dependabot groups updates.
- Manual Release dispatch is main-only; dry-run defaults on. Checks precede a
  separate read-only build job, which stamps both Cargo files, builds with
  `--locked` and verifies a mounted read-only DMG without executing its binary.
- Cargo and plist versions remain numeric; About and tags use `vYY.M.N`.
  Canonical tags determine the next zero-based build within each UTC month.
- Only the optional final publisher has `contents: write`. It downloads the
  verified artifact, rechecks SHA-256, creates a non-overwriting tag at the exact
  checked commit, and publishes one DMG with checksum and generated notes.
  No checkout or repository script executes with the write token.
- Keep traces, screenshots, videos and reports after browser failures for seven
  days. No browser or analysis tooling is shipped inside Rake.app.

## Consequences

Release and dry-run do the same validation; dry-run never creates a tag/release.
Publication failures after tag creation may leave an unused tag; subsequent runs
advance rather than rewriting it. Audits need network access and may uncover new
upstream issues. Current informational GTK-tree advisories do not affect the
macOS build, and Linux application support remains untested. Full keyboard/dialog
acceptance is still separate from automated checks. Signing/notarisation remain
deferred as in ADR 14; neither this decision nor a PR authorises a release.
