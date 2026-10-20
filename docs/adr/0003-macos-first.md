# 3. Target macOS first

Date: 2026-10-19 · Status: Accepted

## Context
The author develops on macOS. Drop-down terminal behavior (window levels, Spaces, focus) is highly OS-specific. Cross-platform support is wanted eventually, but not yet.

## Decision
Develop and test on macOS only. Choose libraries that also support Linux and Windows. Keep platform-specific code behind `cfg(target_os)` in small functions. Do not claim other platforms work until they have been tested.

## Consequences
We move faster now and avoid untested abstractions. Adding a platform later means writing and testing that platform's code, and it may surface differences (for example, Wayland global hotkeys) that call for design changes.
