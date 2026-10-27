#!/bin/sh
# Assemble Rake.app and a DMG from an already built release binary. Prints the DMG path.
# Usage: scripts/package.sh [version]   (defaults to the version in Cargo.toml)
set -eu

cd "$(dirname "$0")/.."

version="${1:-$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml)}"
version="${version#v}"
if [ -z "$version" ]; then
    echo "no version: pass one or set it in Cargo.toml" >&2
    exit 1
fi

binary="target/release/rake"
if [ ! -x "$binary" ]; then
    echo "build first: $binary is missing" >&2
    exit 1
fi

arch="$(uname -m)"
app="dist/Rake.app"
staging="dist/.dmg"
dmg="dist/Rake-v${version}-${arch}.dmg"

rm -rf "$app" "$staging"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$staging"
cp "$binary" "$app/Contents/MacOS/rake"
cp assets/icons/Rake.icns "$app/Contents/Resources/Rake.icns"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>Rake</string>
  <key>CFBundleName</key><string>Rake</string>
  <key>CFBundleExecutable</key><string>rake</string>
  <key>CFBundleIdentifier</key><string>com.jonathantobe.rake</string>
  <key>CFBundleIconFile</key><string>Rake</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>© 2026 Jonathan Tobe</string>
</dict>
</plist>
EOF

plutil -lint "$app/Contents/Info.plist" >/dev/null
# Ad-hoc signature: enough to run locally; not notarised, so downloads still need
# right-click Open until there is a Developer ID.
codesign --force --sign - "$app"

# Drag-to-install layout.
ditto "$app" "$staging/Rake.app"
ln -s /Applications "$staging/Applications"
hdiutil create -quiet -volname Rake -srcfolder "$staging" -ov -format UDZO "$dmg"
rm -rf "$staging"

echo "$dmg"
