# 12. The panel belongs to a display, not a Space

Date: 2026-10-26 · Status: Accepted

## Context
Milestones 1 to 5 put the panel on whichever display macOS chose and tied it to the Space where it was created. A drop-down terminal is summoned while working, so it has to appear where the user is looking, and switching desktops must not make the hotkey silently do nothing. macOS ties windows to Spaces unless told otherwise, and multi-display geometry needs care because AppKit measures in points from the bottom left while the window APIs use physical pixels from the top left.

## Decision
- On every show, from the hotkey or the menubar icon, the panel takes the display under the mouse and keeps it until hidden. The next show chooses again.
- A point in the gap between displays chooses the nearest display.
- The panel joins all Spaces and full-screen auxiliaries, so on its display it is available on every desktop and beside full-screen apps.
- Size and position are computed in physical pixels from that display's frame and usable frame, so its menubar or notch is respected. The webview rasterises at the display's own scale; nothing is rendered at one scale and downsampled to another.
- The AppKit frames are flipped and scaled in one pure function (`display_from_frame`) and the choice of display is another (`screen_at`), both unit tested. The AppKit glue only gathers screens and the pointer.
- Left-clicking the menubar icon toggles the panel; right-click keeps the native About and Quit menu.

## Consequences
- The panel follows the mouse, not keyboard focus. With the pointer parked on another display, the panel appears there. Sticky or focus-following modes would be configuration additions.
- Joining all Spaces means the panel is present on every desktop, including in Mission Control, and over full-screen apps on the same display.
- Showing on another display resizes the window for that display, so the terminal refits and its grid changes with the display.
- Verified against a single display. Multi-display, Spaces and full-screen behaviour need a hand check.
