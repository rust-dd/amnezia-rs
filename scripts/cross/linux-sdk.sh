#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$script_dir/../.." && pwd)"
mkdir -p "$root/target"
if [[ "$#" -eq 0 ]]; then
    stage="$(mktemp -d "$root/target/amnezia-linux-sdk-XXXXXX")"
else
    stage="$(cd "$1" && pwd -P)"
    case "$stage" in
        "$root"/target/amnezia-linux-sdk-*) ;;
        *) echo "Only an SDK previously created in this checkout can be refreshed" >&2; exit 2 ;;
    esac
    [[ -f "$stage/packages.sha256" ]] || { echo "SDK receipt is missing" >&2; exit 2; }
fi
sysroot="$stage/sysroot"
mkdir -p "$stage/downloads" "$sysroot"

while read -r expected package; do
    archive="$stage/downloads/${package##*/}"
    curl --fail --silent --show-error --location --retry 3 \
        "https://deb.debian.org/debian/$package" --output "$archive"
    actual="$(shasum -a 256 "$archive")"
    [[ "${actual%% *}" == "$expected" ]] || { echo "Checksum failed: $package" >&2; exit 1; }
    payload="$(ar -t "$archive" | sed -n '/^data\.tar\./p')"
    [[ "$payload" == data.tar.xz || "$payload" == data.tar.zst ]] || {
        echo "Unexpected Debian archive layout: $package" >&2
        exit 1
    }
    ar -p "$archive" "$payload" | tar -xf - --no-same-owner -C "$sysroot"
done < "$script_dir/linux-packages.sha256"

while IFS= read -r link; do
    destination="$(readlink "$link")"
    if [[ "$destination" == /* ]]; then
        ln -sfn "$sysroot$destination" "$link"
    fi
done < <(find "$sysroot" -type l)

cp "$script_dir/linux-packages.sha256" "$stage/packages.sha256"
echo "Linux build-only SDK: $sysroot"
echo "No package maintainer scripts were executed; no host libraries were installed."
