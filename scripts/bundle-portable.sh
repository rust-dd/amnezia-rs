#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
platform="${1:?Usage: bundle-portable.sh windows|linux}"
case "$platform" in
    windows) triple=x86_64-pc-windows-gnu; executable=amnezia.exe ;;
    linux) triple=x86_64-unknown-linux-gnu; executable=amnezia ;;
    *) echo "Expected windows or linux; use bundle-mac.sh for macOS" >&2; exit 2 ;;
esac
target="${CARGO_TARGET_DIR:-$root/target}"
[[ "$target" == /* ]] || target="$root/$target"
binary="$target/$triple/release/$executable"
[[ -f "$binary" ]] || { echo "Build $triple first" >&2; exit 2; }
stage="$(mktemp -d "$target/amnezia-portable-XXXXXX")"
name="amnezia-preview-$platform-x86_64"
package="$stage/$name"
mkdir -p "$package/assets" "$package/Notices"
cp "$binary" "$package/$executable"
chmod +x "$package/$executable"
rsync -a --exclude='*.mid' --exclude='.DS_Store' "$root/assets/" "$package/assets/"
cp "$root/README.md" "$package/README.md"
cp "$root/amnezia/fonts/LICENSE.txt" "$package/Notices/Font.txt"
cp "$root/amnezia-convert/assets/soundfont/LICENSE.txt" "$package/Notices/GeneralUser-GS.txt"
git -C "$root" rev-parse HEAD > "$package/Revision.txt"
file "$binary" > "$stage/binary-format.txt"
case "$platform" in
    windows) grep -q 'PE32+ executable.*x86-64' "$stage/binary-format.txt" ;;
    linux) grep -q 'ELF 64-bit.*x86-64' "$stage/binary-format.txt" ;;
esac
differences="$(rsync -rcn --itemize-changes --exclude='*.mid' --exclude='.DS_Store' "$root/assets/" "$package/assets/")"
[[ -z "$differences" ]] || { echo "Asset copy failed: $differences" >&2; exit 1; }
(
    cd "$package"
    find . -type f ! -name SHA256SUMS -exec shasum -a 256 {} + > SHA256SUMS
    shasum -a 256 --check SHA256SUMS > "$stage/checksums.txt"
)
if [[ "$platform" == windows ]]; then
    archive="$stage/$name.zip"
    (cd "$stage" && zip -qr "$archive" "$name")
    unzip -tq "$archive"
else
    archive="$stage/$name.tar.gz"
    COPYFILE_DISABLE=1 tar -czf "$archive" -C "$stage" "$name"
    tar -tzf "$archive" > "$stage/archive-files.txt"
fi
(cd "$stage" && shasum -a 256 "${archive##*/}" > "${archive##*/}.sha256")
echo "Preview archive: $archive"
echo "Evidence: $stage"
echo "No tag, upload or release was created. Runtime acceptance on $platform is still required."
