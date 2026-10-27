# Icons

One drawing, two scales: a filled bar (the menubar edge) with an outlined pane
hanging under it. In the 22-unit space the bar is `x 3.5..18.5, y 4.5..6.9`
(radius 1.2), the pane is `x 5.5..16.5, y 9..18` (radius 1.8, stroke 1.6), so the
gap is 2.1. The app icon is that drawing scaled 40 times inside the squircle.

The accent is amber `#e9a13b` on charcoal `#1c1c1c`, with the pane in `#d0d0d0`
(the terminal's foreground colour).

| File | What it is |
|---|---|
| `menubar.svg` | Template source: one colour, transparent background, 22-unit grid. |
| `menubar-22.png` | Template at 1x, black with alpha. |
| `menubar-44.png` | Template at 2x. |
| `app.svg` | App icon source, 1024 canvas. |
| `app-1024.png` | App icon master. |
| `Rake.icns` | App icon set, 16 to 1024. |

macOS tints a template image itself: black on a light menubar, white on a dark
one, so there is no white asset to keep.

`src/lib.rs` (`build_icon_rgba`) draws the same geometry at 44x44 pixels, twice
the 22-unit grid, so Retina screens get real pixels; `tray-icon` scales it to 22
points and marks it as a template. The committed PNGs remain the visual
reference, and the `.icns` is unused until Rake ships as an `.app` bundle.
