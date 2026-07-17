#!/bin/sh
# crunchit installer — detects OS/arch, downloads the latest release binary.
#   curl -fsSL https://raw.githubusercontent.com/artisenalcode/crunchit/main/install.sh | sh
# Override the install directory with CRUNCHIT_INSTALL_DIR (default: ~/.local/bin).
set -eu

REPO="artisenalcode/crunchit"
INSTALL_DIR="${CRUNCHIT_INSTALL_DIR:-$HOME/.local/bin}"

os=$(uname -s)
arch=$(uname -m)
case "$os-$arch" in
    Linux-x86_64)               target="x86_64-unknown-linux-gnu" ;;
    Darwin-arm64)               target="aarch64-apple-darwin" ;;
    Darwin-x86_64)              target="x86_64-apple-darwin" ;;
    *)
        echo "error: no prebuilt binary for $os/$arch." >&2
        echo "Build from source instead: cargo install --git https://github.com/$REPO" >&2
        exit 1
        ;;
esac

tag=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)
[ -n "$tag" ] || { echo "error: could not resolve latest release tag" >&2; exit 1; }

url="https://github.com/$REPO/releases/download/$tag/crunchit-$tag-$target.tar.gz"
echo "Installing crunchit $tag ($target) to $INSTALL_DIR"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$url" -o "$tmp/crunchit.tar.gz"
tar -xzf "$tmp/crunchit.tar.gz" -C "$tmp"

mkdir -p "$INSTALL_DIR"
install -m 755 "$tmp/crunchit" "$INSTALL_DIR/crunchit"

echo "Installed: $("$INSTALL_DIR/crunchit" --version)"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo "note: $INSTALL_DIR is not in your PATH. Add this to your shell profile:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac
