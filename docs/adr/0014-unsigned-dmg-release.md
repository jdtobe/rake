# 14. Release as an unsigned DMG

Date: 2026-10-27 · Status: Accepted

## Context
Releases shipped a `.tar.gz` containing the bare binary. A macOS user expects an application they can drag into Applications, with an icon, a bundle identifier for About and single-instance behaviour, and a LaunchServices identity. Signing and notarisation need an Apple Developer account and were explicitly deferred.

## Decision
- `scripts/package.sh` assembles `dist/Rake.app` (binary, `Info.plist`, `Rake.icns`) and wraps it in `dist/Rake-v<version>-<arch>.dmg` with an Applications symlink for drag-to-install. It runs locally and in CI from the same script.
- The bundle is menu-bar-only (`LSUIElement`), high-resolution capable, identified as `com.jonathantobe.rake`, with the stamped version in both version keys. No minimum macOS version is claimed.
- The bundle is ad-hoc signed (`codesign --sign -`). There is no Developer ID, hardened runtime or notarisation yet.
- The release workflow stamps the version, builds, packages and publishes the DMG instead of the tarball. Only arm64 is built, because the CI runner is Apple Silicon.
- The image is compressed (`UDZO`) with no background image or custom window layout yet.

## Consequences
- A downloaded DMG is quarantined; the first launch needs Finder's right-click **Open** (or clearing the quarantine attribute). The README says so. This is the price of shipping unsigned, and it will look alarming to some users.
- There is no auto-update; releases stay manual.
- Intel Macs cannot run the release build until a universal binary is built.
