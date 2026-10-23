# 9. Adopt the macOS Terminal default profile for colours and font

Date: 2026-10-22 · Status: Accepted

## Context
A terminal is a place people already have a look they like, and Rake's own first-look theme was an arbitrary dark grey. Asking every new user to copy a colour scheme into `config.toml` would make Rake look worse than the terminal they already use, and would add configuration before there is a settings UI. Most macOS users have a Terminal profile they chose; adopting it makes Rake feel native with no setup.

## Decision
- At startup, read the macOS Terminal default profile: `Default Window Settings` in the `com.apple.Terminal` preferences.
- Adopt its background, text and cursor colours, all 16 ANSI colours, and its font family and point size (converted to CSS pixels).
- Leave opacity, blur, background images, selection colour and cursor shape alone: they are window or renderer features Rake does not have yet.
- If the profile cannot be read, or any part of it cannot be decoded, fall back to Rake's own dark look. On other platforms there is nothing to read and the same fallback applies.
- Read once at startup. Profile changes take effect on the next launch, like `config.toml`.
- The theme reaches the page as a JSON object injected into the bundled HTML, so the page needs no extra IPC and no remote requests.

## Consequences
Rake looks like the terminal its user already chose, including Afterglow, with no configuration. Users who change their Terminal profile must restart Rake to see it, and Rake-only customisation is not possible yet. A future settings UI or `theme` config section would extend this decision rather than replace the reader: explicit configuration should win over the adopted profile.
