#!/usr/bin/env bash
set -euo pipefail

[[ "$(uname -s)" == Darwin ]] || { echo "macOS is required" >&2; exit 2; }
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/.." && pwd)"
target="${CARGO_TARGET_DIR:-$root/target}"
[[ "$target" == /* ]] || target="$root/$target"
mkdir -p "$target"
target="$(cd "$target" && pwd)"
cd "$root"

echo "Building the locked release"
cargo build -p amnezia --release --locked
package="$(cargo pkgid -p amnezia --locked)"
version="${package##*#}"
version="${version##*@}"
stage="$(mktemp -d "$target/amnezia-bundle-XXXXXX")"
app="$stage/Amnézia.app"
resources="$app/Contents/Resources"
mkdir -p "$app/Contents/MacOS" "$resources/assets" "$resources/Notices"
cp "$target/release/amnezia" "$app/Contents/MacOS/amnezia"
chmod +x "$app/Contents/MacOS/amnezia"
rsync -a --exclude='*.mid' --exclude='.DS_Store' "$root/assets/" "$resources/assets/"
cp "$root/scripts/mac/Info.plist" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $version" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$app/Contents/Info.plist"
cp "$root/README.md" "$resources/README.md"
cp "$root/amnezia/fonts/LICENSE.txt" "$resources/Notices/Font.txt"
cp "$root/amnezia-convert/assets/soundfont/LICENSE.txt" "$resources/Notices/GeneralUser-GS.txt"

plutil -lint "$app/Contents/Info.plist"
differences="$(rsync -rcn --itemize-changes --exclude='*.mid' --exclude='.DS_Store' "$root/assets/" "$resources/assets/")"
[[ -z "$differences" ]] || { echo "Asset copy verification failed: $differences" >&2; exit 1; }
(
    cd "$resources/assets"
    find . -type f -exec shasum -a 256 {} + > ../Assets.sha256
)
(
    cd "$stage"
    "$app/Contents/MacOS/amnezia" --check-installation > installation.txt
)
expected_assets="$(cd "$resources/assets" && pwd -P)"
grep -F -x "Assets: $expected_assets" "$stage/installation.txt"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"

destination="$target/Amnézia.app"
if [[ -e "$destination" || -L "$destination" ]]; then
    mkdir "$stage/previous"
    mv "$destination" "$stage/previous/Amnézia.app"
    echo "Previous bundle retained at $stage/previous/Amnézia.app"
fi
mv "$app" "$destination"
echo "Bundle: $destination"
echo "Build evidence: $stage/installation.txt"
echo "Ad-hoc signed for local testing; not notarized or validated on another machine."
