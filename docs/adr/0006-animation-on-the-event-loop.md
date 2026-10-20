# 6. Drive the slide animation from the event loop, with pure maths

Date: 2026-10-19 · Status: Accepted

## Context
The window should slide in from the top and back out. We needed a way to run frames without a second thread or a busy loop, keep the app idle when nothing is moving, and make the behavior testable without a GUI.

## Decision
- **Pure maths:** `src/animation.rs` holds `ease_out_quad` and `Slide`. A `Slide` is a from/to y position, a start time and a duration. Callers pass the current time in, so tests need no clock or window.
- **Event loop frames:** while a slide runs, the loop sets `ControlFlow::WaitUntil(now + 16ms)` and `DropDown::tick` moves the window on each wake-up (`MainEventsCleared`). When no slide is running the loop goes back to `ControlFlow::Wait`, so the app uses no CPU while idle.
- **Reversal:** toggling mid-slide starts a new `Slide` from the window's current y. The duration is scaled by the remaining distance, so a reversal does not jump and does not feel slow.
- **Slide, not resize:** the window moves from fully above the monitor's top edge to its resting spot, rather than growing in height.
- **Hide at the end:** the window is hidden with `set_visible(false)` only when the slide out finishes, then parked at its resting spot so the next show finds the same monitor.
- **Config:** `animation_ms` (default 300, `0` to turn it off, at most 2000).

## Consequences
- Easy to test and no extra threads. The frame rate is bounded by the event loop's timer, about 60 frames per second.
- Moving a window every frame is not as smooth as a compositor-driven animation. If it looks rough, the next step is a native `NSAnimationContext` animation through `objc2`.
- If macOS refuses to place the borderless window partly above the screen, the fallback is to animate the height from 0 instead. That would only change what `DropDown::tick` applies, not `Slide`.

## Amendment (same day)
The first version used `ease_out_cubic` with a 200 ms default. In use, it looked like 100 ms or less, because a cubic ease-out has covered about 88% of the distance by half time. We switched to `ease_out_quad` (75% at half time) and a 300 ms default so the slide is visible. `animation_ms` remains the knob for taste.

## Amendment 2: why the slide looked instant, and what we changed
After the first amendment the slide still looked instant. Measuring (not guessing) showed three separate causes, found by logging the requested and OS-reported window position and every event, with timestamps, from a temporary timer-driven toggle:

1. **macOS clamped the window below the menubar.** A floating-level window cannot be placed above the menubar, so every requested position above y=60 was reported back as y=60 and the whole off-screen part of the slide collapsed into one spot. We now raise the window to `NSStatusWindowLevel` (25) so it can start fully above the screen, and rest it just below the menubar (`top_inset` in `geometry::window_rect`, read from `NSScreen`).
2. **Timer wake-ups arrived late or not at all.** `ControlFlow::WaitUntil(now + 16ms)` was repeatedly woken late: gaps of 26 to 590 ms, during which the loop slept and the slide did not move. The earlier "hide" slide only looked fine because each `set_outer_position` produced `Moved` events that woke the loop anyway. While a slide runs we now use `ControlFlow::Poll`. Slide positions come from elapsed time, so the spin rate does not matter, and the loop returns to `Wait` (0% CPU) when idle.
3. **Not a cause, ruled out:** painting (1 to 10 ms), `set_visible` and `set_focus` (under 6 ms), and macOS's window-appear animation (disabling it made no difference).

Known remaining flaw: a 40 to 70 ms hitch in the first ~60 ms of each slide in, while the window is ordered in and resized. The OS-driven alternative, a display-synchronised callback (`CVDisplayLink`), is the next step if this is noticeable.

How to measure again: temporarily add a toggle sent from a thread through the event-loop proxy, and log `Instant::now()`, the requested y, and `window.outer_position()` on every `DropDown::tick`. Check the maximum gap between ticks and that the reported y follows the requested y.
