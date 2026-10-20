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
