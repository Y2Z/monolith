#!/bin/sh
# Builds target/Monolith.app and target/Monolith-macos.zip (macOS only).
# With BIN unset it uses target/release/monolith-gui (cargo build --release
# --bin monolith-gui --features gui); the CD workflow passes a universal
# (Intel + Apple silicon) binary made with lipo.
# Ad-hoc signed, not notarized: the first time, open it with right-click → Open.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)
BIN=${BIN:-$root/target/release/monolith-gui}
app="$root/target/Monolith.app"
iconset="$root/target/monolith.iconset"

rm -rf "$app" "$iconset"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$iconset"

cp "$BIN" "$app/Contents/MacOS/monolith-gui"
sed "s/@VERSION@/$version/g" "$root/dist/macos/Info.plist" > "$app/Contents/Info.plist"

# Icon: render every size iconutil wants from the large source PNG
for size in 16 32 128 256 512; do
    sips -z $size $size "$root/assets/icon/icon.png" \
        --out "$iconset/icon_${size}x${size}.png" > /dev/null
    double=$((size * 2))
    sips -z $double $double "$root/assets/icon/icon.png" \
        --out "$iconset/icon_${size}x${size}@2x.png" > /dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/monolith.icns"
rm -rf "$iconset"

# Apple silicon refuses to run unsigned code; an ad-hoc signature is enough
codesign --force --deep --sign - "$app"

cd "$root/target"
rm -f Monolith-macos.zip
ditto -c -k --keepParent Monolith.app Monolith-macos.zip
echo "target/Monolith-macos.zip"
