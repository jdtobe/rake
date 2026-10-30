# Tabs implementation decisions and review

These notes preserve execution rulings without requiring readers to reconstruct
the scratch ledger. Approved behaviour remains in the [design](../superpowers/specs/2026-10-29-tabs-design.md).

## Cost and review

Implementation was inline in the existing isolated worktree. Two focused agent
tasks were used: native process inspection/cleanup, and a final routing/lifecycle
review. The parent agent read the native implementation, reran its tests, assessed
the final findings, implemented fixes and performed final verification. There were
no per-task reviewer agents or repeated full-plan implementation agents.

## Rulings made

| Decision | Reason | Cost if wrong / remaining boundary |
|---|---|---|
| Parent owns final review; focused agents provide evidence, not acceptance | User requested lower cost and parent accountability | Author blind spots remain possible; actual native acceptance is still required |
| Implement independent config/transport tasks while native adapter runs | Avoid idle time and repeated contexts | Concurrent red phases temporarily block whole-suite builds; final integrated suite is authoritative |
| Keep old single-terminal wiring until workspace integration, then remove it | Preserve runnable intermediate commits | A second queue would be unsafe if retained; obsolete unbounded bridge and JS compatibility were removed |
| Keep `focus_session(id)` alongside active-terminal `focus()` rather than overload | Rust does not overload methods by argument count; panel show needs current focus | Callers must distinguish explicit session focus from active-terminal focus |
| Test restore fallback/recovery in the workspace, not the PTY manager | Policy belongs in one owner | Real per-resource spawn failures remain possible; current tests cover missing shell, missing cwd and empty recovery |
| Test native short-read cancellation without assuming the exact byte cap is reached | Event slots can fill before byte credits | Deterministic transport tests separately prove the 256-KiB cap and cancellation |
| Verify native menus after integration using a temporary AppKit harness | Unconnected menus cannot prove app wiring | Synthetic action/accelerator checks are not physical-keyboard/dialog appearance acceptance |
| Replace source-text ready/input/paste assertions with real WebKit IPC tests | Behaviour matters, not JS formatting | Native system clipboard still needs hands-on acceptance |
| Use Node 22 for Playwright provisioning | Node 26 hung in extraction twice; Node 22 succeeded | Browser tooling needs a compatible runtime; Node 22 is pinned in CI |
| Validate cwd explicitly before portable-pty spawn | Dependency silently falls back for invalid directories | Directory deletion can still race spawn; later native observations refresh saved cwd |
| Cleanup off UI thread; wait before exiting; report failures rather than claim termination | Avoid freezing the panel or exiting before workers finish | Cleanup errors abort quit with a notice; detached/reparented daemons remain outside scope |
| Treat terminal failure separately from confirmed process exit | A failed output bridge can leave real apps alive | Failed tabs disable input but still perform native close/quit job checks |
| Use a short isolated test HOME for bundle smoke tests | Existing Unix socket pathname limit rejected the longer approved temporary prefix | Long-HOME socket support is pre-existing and not changed by tabs |
| Preserve saved layout after incomplete startup until an intentional workspace edit | Correcting a failed shell should recover previous names/locations | Successful new-tab/manual rename/close actions establish the replacement layout |
| Do not replace tao's application delegate to intercept OS shutdown | No public termination-veto hook; avoid intrusive lifecycle replacement | Logout/shutdown, Force Quit and external signals cannot guarantee confirmation |

## Final review findings

1. **Important: asynchronous selection replies cancelled rename editors.**
   Parent reproduced the failure in WebKit. Keyed tab DOM and focus preservation
   fix it. `rename_survives_async_selection_round_trips` failed before the fix and
   passes for immediate and delayed replies; repeated rename/reset tests also pass.
2. **Important: failed startup overwrote the recoverable layout.**
   Parent reproduced the file replacement with a missing shell. Saving is held
   after incomplete startup, including quit. `failed_startup_does_not_overwrite_recoverable_layout`
   failed before the fix and passes afterward.

Final verification exposed two test-synchronization problems: native tree
snapshots can be legitimately incomplete while a pipeline forks, and parallel
fixture names needed a counter rather than relying only on clock uniqueness.
Tests now wait for a stable snapshot within their original deadline and use
collision-free fixture names. Three consecutive full Rust suites passed afterward.

No reviewer minors were deferred. Tab reordering is wanted future product work,
not a review defect. The [acceptance checklist](tabs.md) explicitly lists remaining
physical-keyboard, dialogs, SSH and multi-display checks. No merge, push or release
publication was performed for this implementation.
