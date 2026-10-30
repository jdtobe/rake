# Workflow Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement inline, task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Strong, non-duplicated CI and a manual-only, verified calendar-versioned DMG release.

**Architecture:** One reusable check workflow serves PRs, main pushes and releases. Dependency audits can also run weekly. A separate read-only build job produces a verified DMG; only the final optional publish job has write permission.

**Tech Stack:** GitHub Actions, existing Rust/Node 22/WebKit tests, Python standard-library script tests, ShellCheck, actionlint, cargo-audit, npm audit and native macOS packaging tools.

**Spec:** The user's approved eight-point hardening proposal in chat: shared checks without duplicate PR runs; Rust/npm security audits plus weekly scans; workflow/shell linting; SHA-pinned Actions and Dependabot; version tests; package verification; failed browser artifacts; manual release with dry-run. One DMG release asset containing Rake.app and an Applications symlink, `vYY.M.N` numbering starting at zero each UTC month, checksum in release notes, unsigned/ad-hoc arm64 distribution.

## Global Constraints
- Retain the user's lower-cost inline execution preference; no new implementer/reviewer agents needed for this small extension.
- No merge or release dispatch/publication during implementation. Open the requested PR only after local verification.
- Normal releases publish only via workflow_dispatch on main; dry_run does not create tags or releases.
- Numeric Cargo/bundle version, v-prefixed displayed/tag version. No dependency updates without examining actual audit findings.
- Keep all new tooling development-only and install locally into ignored scratch, not global user tools.

## Review Focus
- Tag parsing must ignore unrelated/noncanonical tags and reset at UTC month/year boundaries (Task 1).
- Stamping must not alter dependency versions, leave mismatched root lock metadata or accept invalid calendar versions (Task 1).
- A valid-looking DMG must actually contain the right executable, versions, icon and Applications link (Task 2).
- PR code must never receive release write permission, and dry-run must never publish (Task 3).
- Failure artifacts must be available while normal PR/main runs avoid duplicated macOS jobs (Task 3).

## Task 1: Test and harden version calculation/stamping
**Files:** `.github/scripts/next-version.sh`, new `.github/scripts/stamp-version.py`, `tests/ci/test_version.py`, `src/lib.rs`.
**Interfaces:** next-version.sh [YYYY-MM] defaults UTC; stamp-version.py TAG updates package/root-lock versions in current directory after validating both. About displays `v` plus Cargo version.
- [ ] Write failing unittest subprocess cases for first=26.10.0, existing 0/2→3, unrelated/malformed tags ignored, November/year resets, invalid periods rejected, package-only stamp and no mutation on invalid lock/tag. Change About assertion before production formatting.
- [ ] Run tests to red; implement scripts and display change. Run Python tests, Rust tests, clippy/fmt. Commit the logical version change.

## Task 2: Verify the actual packaged image
**Files:** `scripts/package.sh`, new `scripts/verify-package.sh`, `tests/ci/test_package.py`.
**Interfaces:** verify-package.sh DMG NUMERIC_VERSION exits nonzero on any invalid content; safely mounts readonly with trap cleanup. Packaging validates numeric version before file operations.
- [ ] Add failing tests for invalid package version and missing verifier, then build a development DMG and test valid image, wrong expected version and corrupt image against the real tools. Gate macOS-only cases explicitly outside macOS, not by pretending they passed.
- [ ] Verify checksum, plist, executable arm64 architecture, icon, ad-hoc signature, numeric version keys and /Applications target inside mounted image. Do not launch downloaded executables as a verification shortcut.
- [ ] Run package tests, Rust suite and strict script lint. Commit packaging verification.

## Task 3: Shared, pinned and least-privilege workflows
**Files:** `.github/workflows/{ci,checks,security,release}.yml`, remove obsolete `pr.yml`, `.github/dependabot.yml`, new `.github/scripts/install-analysis-tools.sh`, `playwright.config.js`, `README.md`, new `docs/adr/0016-verified-manual-releases.md`.
- [ ] Resolve Action tags to real commit SHAs and verify analysis-tool archive hashes against GitHub release asset digests. Pin cargo-audit version. Local installs go to this plan's ignored workspace.
- [ ] Shared checks: Ubuntu workflow/shell lint + Python tests, macOS fmt/strict Clippy/all Rust+doc tests/WebKit tests/development DMG verification, dependency-audit reusable job. CI entry runs PR/main/manual only. Audits also weekly and manual; read-only tokens, no pull_request_target.
- [ ] Release guards main before checks; build --locked after stamping both manifest/lock; assert arm64; verify DMG before uploading workflow artifact. dry_run skips the sole contents:write publisher. Publish exactly the DMG to the immutable checked commit; checksum in generated release notes. Serialize releases, do not auto-release on push/tag.
- [ ] Retain browser trace/screenshot/video on failure and upload test-results/report artifacts on failed macOS checks. Add npm lockfile Dependabot and group updates to reduce PR noise.
- [ ] Run actionlint, ShellCheck, Python script tests, cargo-audit and npm audit, all Rust/WebKit checks, release build and real DMG validation. Record any actual audit/tool blockers rather than hiding them. Review token flow and publishing condition personally.
- [ ] Document exact triggers, manual/dry-run instructions and version semantics; commit. Push a dedicated remote feature branch and open the requested PR; do not merge or dispatch Release.

## Self-review
All eight approved additions map to tasks above. Main/PR shared code remains read-only; publish permissions are isolated. Version/packaging scripts are tested through subprocesses and native mounted images, not source-text assertions. Existing native hands-on tabs acceptance remains separate and unverified.
