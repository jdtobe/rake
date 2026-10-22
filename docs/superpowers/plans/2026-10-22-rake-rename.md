# Rake Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the provisional `quake-terminal` identity with the Rake brand everywhere, including the package, executable, UI metadata, and config path.

**Architecture:** Add one canonical `APP_NAME` for human-facing text while Cargo supplies the lowercase package and binary name. Supersede, rather than edit, the accepted config-location ADR.

**Tech Stack:** Rust 2021, Cargo, Markdown ADRs

**Spec:** `docs/superpowers/specs/2026-10-22-rake-terminal-design.md`

## Global Constraints

- Product and human-facing name: `Rake`.
- Cargo package, binary, and library crate: `rake`.
- Config path: `~/.config/rake/config.toml`.
- Do not retain or migrate the old config path.
- Do not edit accepted ADRs; supersede ADR 0005's location decision with a new ADR.
- Descriptive “Quake-style” wording may remain; old `quake-terminal` and `quake_terminal` identifiers may not.
- Stable Rust 2021; macOS is the only supported/tested platform.

## Review Focus

- Human-facing metadata must say `Rake`, not lowercase `rake`; pinned by the `about_metadata_uses_rake_brand` test below.
- The executable/library rename must not leave stale Rust imports; pinned by the full build and test run below.
- Config discovery must use only `~/.config/rake/config.toml`; pinned by `config_path_uses_rake_directory`.
- Old accepted ADR text must remain historically intact; pinned by reviewing the staged file list before commit.
- Generic “Quake-style” product-description text must not be mistaken for the old product identifier; the identifier grep below intentionally targets only hyphenated/underscored forms.

---

### Task 1: Rename the package, application metadata, and configuration

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/dropdown.rs`
- Modify: `src/config.rs`
- Modify: `README.md`
- Modify: `docs/design.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/open-questions.md`
- Create: `docs/adr/0007-rake-name-and-config-path.md`
- Delete: `HANDOFF.md`
- Test: unit tests in `src/lib.rs` and `src/config.rs`

**Interfaces:**
- Consumes: existing `about_metadata()`, `config_path(&Path)`, and Cargo package metadata.
- Produces: `pub const APP_NAME: &str = "Rake"`; Cargo crate `rake`; config location `$HOME/.config/rake/config.toml`.

- [ ] **Step 1: Change tests first to require the Rake identity**

Rename the metadata test to `about_metadata_uses_rake_brand` and assert `meta.name.as_deref() == Some("Rake")` while retaining the Cargo-version assertion. Rename the config-path test to `config_path_uses_rake_directory` and assert `/home/me/.config/rake/config.toml`.

- [ ] **Step 2: Run the focused tests and verify both fail**

Run: `cargo test about_metadata_uses_rake_brand config_path_uses_rake_directory`

Expected: FAIL because metadata still reports `quake-terminal` and the config path still contains `quake-terminal`. If Cargo accepts only one filter, run each test filter separately.

- [ ] **Step 3: Rename Cargo and Rust identifiers**

Set `package.name = "rake"` and update the package description in `Cargo.toml`; regenerate `Cargo.lock`. Add `pub const APP_NAME: &str = "Rake"` in `src/lib.rs`, use it in About metadata, tray tooltip, and window title, and change `main.rs` imports from `quake_terminal` to `rake`.

- [ ] **Step 4: Rename the config directory and test scratch prefix**

Change `config_path` to join `rake`, update its module documentation, and rename the test temporary-directory prefix to `rake-test`.

- [ ] **Step 5: Run the focused tests and verify they pass**

Run each filter: `cargo test about_metadata_uses_rake_brand` and `cargo test config_path_uses_rake_directory`

Expected: both PASS.

- [ ] **Step 6: Record and document the naming decision**

Create ADR 0007 with status Accepted. State that the product is Rake, Cargo identifiers are `rake`, the config moves to `~/.config/rake/config.toml` without migration, and this supersedes only ADR 0005's config-location decision. Update current-state docs and remove the resolved project-name portion of open question 7 while leaving licensing open. Delete the temporary, already-consumed `HANDOFF.md`; carry its still-relevant animation checks in the approved design and current docs.

- [ ] **Step 7: Verify no old identifiers remain outside historical records**

Run: `rg 'quake-terminal|quake_terminal' --glob '!docs/adr/0005-toml-config-read-at-startup.md' --glob '!docs/superpowers/**'`

Expected: no matches. `Quake-style` may remain as a generic description.

- [ ] **Step 8: Run complete verification**

Run: `cargo test`

Expected: 55 tests pass (or the new total after renamed/added tests), 0 fail.

Run: `cargo clippy --all-targets`

Expected: exit 0 with no warnings.

Run: `cargo fmt -- --check`

Expected: exit 0 with no diff.

Run: `git diff --check`

Expected: no output.

- [ ] **Step 9: Review staged scope and commit**

Confirm ADR 0005 is unmodified and no implementation from the terminal plan is staged.

```bash
git add Cargo.toml Cargo.lock src README.md docs HANDOFF.md
git commit -m "chore: Rake branding - rename package and config path"
```
