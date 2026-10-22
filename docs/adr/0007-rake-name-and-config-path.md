# 7. Use the Rake name and config path

Date: 2026-10-22 · Status: Accepted

## Context
The provisional `quake-terminal` package name was not the product's intended identity and was too close to existing projects. The package name also appeared in the executable, UI metadata and configuration path.

## Decision
- The product and human-facing application name is **Rake**.
- The Cargo package, binary and library crate are named `rake`.
- Configuration is read from `$HOME/.config/rake/config.toml`.
- The old configuration path is not read or migrated while Rake remains an early prototype.

This supersedes only the location decision in [ADR 5](0005-toml-config-read-at-startup.md). Its decisions about TOML, startup-only loading and error handling remain accepted.

## Consequences
Rake has one consistent identity across code, UI and documentation. Existing local prototype configuration must be moved manually to the new directory.
