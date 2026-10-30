# 15. Independent terminal tabs in one webview

Date: 2026-10-29 · Status: Accepted

## Context
Tabs are an explicit priority for independent servers, apps, pipeline stages and
files. A single PTY wired directly to one xterm cannot retain those workspaces.
Closing or quitting must not silently terminate work; title strings cannot prove
that a shell is idle or reveal the authoritative local directory.

## Decision
- Rust owns ordered metadata and monotonically allocated IDs, separately from PTY
  resources. IDs, not indices, address all session input/output and callbacks.
- Keep one WKWebView and one xterm per tab, with WebGL on the selected tab only.
  Inactive tabs retain their buffers and parse output; switching refits only the
  selected grid. DOM rendering remains the fallback.
- Bound queued plus browser-in-flight output to 256 KiB per tab and 64 delivery
  slots, chunk output to at most 8192 bytes and schedule at most 64 KiB per tab per
  pump. Browser write callbacks acknowledge credit; removal cancels blocked writers.
- Native macOS process identity, cwd, descendants and foreground groups inform
  local directory inheritance and close/quit confirmation. Unknown state prompts.
  I/O failure is not evidence of process exit. Cleanup identity-checks local jobs,
  escalates within a deadline off the UI thread, and quit waits for completion.
- All Rake-owned quit controls route through one policy; tao's lack of a public
  termination veto means OS logout/shutdown and external termination are outside
  the confirmation guarantee. Do not replace its delegate in this milestone.
- Bottom tabs, current-directory inheritance, fresh-tab startup, fresh replacement
  after last close and System appearance are defaults, configurable in TOML.
  Terminal profile colours and font remain separate from UI appearance.
- Restore only versioned names/order/selection/local paths, with fresh shells;
  write `tabs.json` atomically beside the existing instance lock, outside config.
  Store path bytes losslessly and keep manual names distinct from automatic ones.
- Tab drag reordering is wanted follow-up; stable IDs already avoid tying identity
  to ordering. General Settings UI and profiles are outside this milestone.

## Consequences
There is one shell/terminal buffer per tab, not one shared screen. Heavy output
can block its producing session rather than growing unbounded queues. Process
inspection has inherent races; intentionally detached/reparented daemons are not
cleanup targets. Restored shells still use their own normal history files.

Automated tests cover Rust session policy/native jobs and a real Playwright WebKit
document. Physical-keyboard, dialog appearance, SSH and multi-display acceptance
remain explicit hand checks; see `docs/testing/tabs.md`.
