# 5. TOML config in `~/.config`, read once at startup

Date: 2026-10-19 · Status: Accepted

## Context
The hotkey, and soon the window height and animation time, need to be user-configurable. We had to pick a format, a location, and when changes take effect.

## Decision
- **Format:** TOML, parsed with `serde` and `toml`. Unknown keys are rejected (`deny_unknown_fields`) so typos are reported instead of ignored.
- **Location:** `$HOME/.config/quake-terminal/config.toml`. It is easy to find and edit by hand and works the same on macOS and Linux. We do not use `~/Library/Application Support`.
- **When it applies:** read once at startup. Changes need a restart.
- **Missing file:** use defaults. The app does not create the file.
- **Invalid file:** print an error that names the file and the problem, and exit non-zero. We do not silently fall back to defaults, because a hotkey that quietly doesn't change is hard to debug.
- **Hotkey syntax:** the `global-hotkey` string format, for example `Ctrl+\``, `Alt+Space`, `Cmd+Shift+K`.

## Consequences
- Simple and predictable, with no file watching and no runtime re-registration of hotkeys.
- Users must restart the app after editing. A "Reload Config" menu item can be added later without changing the file format.
- If `HOME` is unset we use defaults. Windows paths are out of scope until we support Windows.
