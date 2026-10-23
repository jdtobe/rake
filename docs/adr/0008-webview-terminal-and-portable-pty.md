# 8. Use a bundled xterm.js webview and portable-pty

Date: 2026-10-22 · Status: Accepted

## Context
Rake needs a complete terminal emulator and renderer without turning its first usable terminal into a custom text-rendering project. It also needs a real login shell connected through a pseudoterminal while preserving the existing single `tao` GUI event loop.

## Decision
- Render and emulate the terminal with xterm.js 6.0.0 and its fit addon 0.11.0 in a `wry` 0.57 webview backed by WKWebView on macOS.
- Bundle the HTML, JavaScript, CSS and required license notices with Rake. Terminal operation does not load remote content.
- Run the shell through `portable-pty` 0.9.
- Read PTY output on a worker thread and forward binary-safe events into the existing `tao` loop. All webview work remains on the GUI thread.
- Buffer output until the page reports readiness and preserve byte and event order through shell exit.

## Consequences
Rake gains mature terminal behavior without implementing an emulator or GPU renderer. It adds a web runtime boundary and bundled JavaScript assets, so the Rust/JavaScript protocol, focus, clipboard behavior and output throughput require explicit tests and macOS verification. A future native renderer would supersede this ADR rather than silently adding a second emulator core.
