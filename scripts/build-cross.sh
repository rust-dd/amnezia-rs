#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
platform="${1:?Usage: build-cross.sh windows|linux|macos [LINUX_SYSROOT]}"
case "$platform" in
    windows)
        command -v x86_64-w64-mingw32-gcc >/dev/null || {
            echo "MinGW-w64 is required for the Windows GNU target" >&2
            exit 2
        }
        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
            cargo build -p amnezia --release --locked --target x86_64-pc-windows-gnu -j 4
        ;;
    linux)
        sysroot="${2:?Run scripts/cross/linux-sdk.sh and pass its sysroot path}"
        sysroot="$(cd "$sysroot" && pwd)"
        for library in alsa libudev wayland-client xkbcommon libffi; do
            [[ -f "$sysroot/usr/lib/x86_64-linux-gnu/pkgconfig/$library.pc" ]] || {
                echo "The pinned Linux SDK is missing $library.pc" >&2
                exit 2
            }
        done
        zigbuild="$root/target/cross-tools/bin/cargo-zigbuild"
        if [[ ! -x "$zigbuild" ]]; then
            zigbuild="$(command -v cargo-zigbuild)" || {
                echo "Install cargo-zigbuild locally as documented in docs/TESTING.md" >&2
                exit 2
            }
        fi
        PKG_CONFIG_ALLOW_CROSS=1 \
        PKG_CONFIG_PATH="" \
        PKG_CONFIG_LIBDIR="$sysroot/usr/lib/x86_64-linux-gnu/pkgconfig" \
        PKG_CONFIG_SYSROOT_DIR="$sysroot" \
        CARGO_ZIGBUILD_CACHE_DIR="$root/target/cross-tools/cache" \
        CARGO_ENCODED_RUSTFLAGS="-Lnative=$sysroot/usr/lib/x86_64-linux-gnu" \
            "$zigbuild" zigbuild -p amnezia --release --locked --target x86_64-unknown-linux-gnu.2.36 -j 4
        ;;
    macos)
        bash scripts/bundle-mac.sh
        ;;
    *) echo "Expected windows, linux or macos" >&2; exit 2 ;;
esac
