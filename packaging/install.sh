#!/usr/bin/env bash
# Build the release binary and install it with its desktop entries and icon.
#
#   packaging/install.sh                 # into ~/.local
#   packaging/install.sh --prefix /usr/local
#   packaging/install.sh --uninstall
#
# The binary embeds its configuration, fonts, textures and coastlines, so it runs from anywhere. It needs
# only the GPU stack (Vulkan or GL) and libc.
set -euo pipefail

prefix="${HOME}/.local"
uninstall=0
while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) prefix="${2:?--prefix needs a directory}"; shift 2 ;;
        --uninstall) uninstall=1; shift ;;
        *) echo "install.sh: unknown argument $1" >&2; exit 2 ;;
    esac
done

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(dirname "$here")"
bin="$prefix/bin/retroradar"
apps="$prefix/share/applications"
icons="$prefix/share/icons/hicolor/scalable/apps"

if [ "$uninstall" = 1 ]; then
    rm -f "$bin" "$apps/retroradar.desktop" "$apps/retroradar-pi.desktop" "$icons/retroradar.svg"
    echo "removed retroradar from $prefix"
    exit 0
fi

# A symlink here is the development launcher (scripts/retroradar-launch); replacing it would break it.
if [ -L "$bin" ]; then
    echo "install.sh: $bin is a symlink (the development launcher?); remove it or pick another --prefix" >&2
    exit 1
fi

if [ -x "$repo/retroradar" ] && [ ! -f "$repo/Cargo.toml" ]; then
    # An unpacked release tarball: the binary is already built.
    built="$repo/retroradar"
else
    if ! command -v cargo >/dev/null 2>&1 && [ -r "$HOME/.cargo/env" ]; then
        # shellcheck source=/dev/null
        . "$HOME/.cargo/env"
    fi
    command -v cargo >/dev/null 2>&1 || { echo "install.sh: cargo not found" >&2; exit 127; }
    (cd "$repo" && cargo build --release --locked -p retroradar)
    built="$repo/target/release/retroradar"
fi
install -Dm755 "$built" "$bin"
install -Dm644 "$here/retroradar.desktop" "$apps/retroradar.desktop"
install -Dm644 "$here/retroradar-pi.desktop" "$apps/retroradar-pi.desktop"
install -Dm644 "$here/retroradar.svg" "$icons/retroradar.svg"
echo "installed $bin"
echo "desktop entries in $apps; make sure $prefix/bin is on PATH"
