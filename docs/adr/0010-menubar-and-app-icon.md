# 10. Menubar and app icon: bar and panel

Date: 2026-10-26 · Status: Accepted

## Context
Rake lives in the menubar, so its mark has to read at 22 points beside system icons and work as a macOS template image: one colour plus alpha, tinted by the system for light and dark menubars. The icon was a solid placeholder square that said nothing about what Rake is.

## Decision
- The mark is **bar and panel**: a filled bar, the menubar edge, above an outlined pane hanging beneath it.
- The geometry sits on a 22-unit grid: the bar spans x 3.5–18.5, y 4.5–6.9 (radius 1.2); the pane spans x 5.5–16.5, y 9–18 (radius 1.8, stroke 1.6); the gap is 2.1. `assets/icons/README.md` records it.
- The menubar asset is a template: black pixels with alpha on a transparent background. No light or dark variants are kept.
- The colour identity, used by the app icon and by platforms that do not tint menubar icons, is amber `#e9a13b` on charcoal `#1c1c1c`, with the pane in `#d0d0d0`.
- The app icon repeats the drawing, scaled 40 times inside a squircle. The SVGs, PNGs and `Rake.icns` are committed; the `.icns` waits for an `.app` bundle.
- `build_icon_rgba` rasterises the geometry at 44x44 pixels, twice the drawing grid, so Retina screens get real pixels; `tray-icon` scales the image to 22 points and marks it as a template.
- The in-code rasteriser mirrors the SVG with 4x4 supersampling. The committed `menubar-44.png` stays the visual reference.

## Consequences
One drawing serves both the menubar and the app icon, and the template rule keeps the menubar icon legible in either appearance. Antialiasing differs slightly from the browser-rendered PNG (a mean alpha difference of about 1/255 on the 44-pixel asset), which is invisible at menubar size. Changing the mark means updating the SVG, the rasteriser and the PNGs together.
