#!/usr/bin/env sh
set -e

REPO="arnonsang/vyn"
BIN="vyn"
INSTALL_DIR="/usr/local/bin"

# Detect downloader
if command -v curl >/dev/null 2>&1; then
  download() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  download() { wget -qO- "$1"; }
else
  echo "error: curl or wget is required" >&2
  exit 1
fi

# Detect OS and arch
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo "error: unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    # Use musl build on Alpine
    if [ -f /etc/alpine-release ]; then
      TARGET="x86_64-unknown-linux-musl"
    fi
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *) echo "error: unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "error: unsupported OS: $OS" >&2
    echo "For Windows, download from https://github.com/$REPO/releases/latest" >&2
    exit 1
    ;;
esac

URL="https://github.com/$REPO/releases/latest/download/$BIN-$TARGET.tar.gz"

echo "Detected: $OS/$ARCH -> $TARGET"
echo "Downloading $BIN from $URL"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

download "$URL" | tar xz -C "$TMP"

# Install (try sudo if needed)
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP/$BIN" "$INSTALL_DIR/$BIN"
else
  sudo mv "$TMP/$BIN" "$INSTALL_DIR/$BIN"
fi

chmod +x "$INSTALL_DIR/$BIN"

echo "Installed $BIN to $INSTALL_DIR/$BIN"

# Record install method in global config so 'vyn update' can suggest the right command.
VYN_GLOBAL_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/vyn"
mkdir -p "$VYN_GLOBAL_DIR"
printf 'install_method = "binary"\n' > "$VYN_GLOBAL_DIR/global.toml"

"$INSTALL_DIR/$BIN" --version
