#!/usr/bin/env sh
set -e

BIN="vyn"
VYN_GLOBAL_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/vyn"
GLOBAL_TOML="$VYN_GLOBAL_DIR/global.toml"

# Read install_method from global.toml if present
install_method=""
if [ -f "$GLOBAL_TOML" ]; then
  install_method="$(grep '^install_method' "$GLOBAL_TOML" | sed 's/.*= *"\(.*\)"/\1/')"
fi

# Fall back to detecting from binary path
if [ -z "$install_method" ]; then
  BIN_PATH="$(command -v "$BIN" 2>/dev/null || true)"
  if [ -n "$BIN_PATH" ]; then
    case "$BIN_PATH" in
      */.cargo/bin/*) install_method="cargo" ;;
      */.local/bin/*|*/usr/local/bin/*) install_method="binary" ;;
    esac
  fi
fi

echo "Detected install method: ${install_method:-unknown}"

remove_binary() {
  target="$1"
  if [ -f "$target" ]; then
    if [ -w "$target" ]; then
      rm "$target"
    else
      sudo rm "$target"
    fi
    echo "Removed $target"
  else
    echo "Binary not found at $target, skipping."
  fi
}

case "$install_method" in
  binary)
    # Try common binary install paths
    for dir in "/usr/local/bin" "$HOME/.local/bin"; do
      if [ -f "$dir/$BIN" ]; then
        remove_binary "$dir/$BIN"
        break
      fi
    done
    ;;
  cargo)
    if command -v cargo >/dev/null 2>&1; then
      cargo uninstall vyn-cli
    else
      # cargo not on PATH, find and remove manually
      remove_binary "$HOME/.cargo/bin/$BIN"
    fi
    ;;
  *)
    # Unknown method: find the binary wherever it is
    BIN_PATH="$(command -v "$BIN" 2>/dev/null || true)"
    if [ -n "$BIN_PATH" ]; then
      remove_binary "$BIN_PATH"
    else
      echo "Could not locate $BIN binary. Remove it manually."
    fi
    ;;
esac

# Remove global config
if [ -d "$VYN_GLOBAL_DIR" ]; then
  rm -rf "$VYN_GLOBAL_DIR"
  echo "Removed $VYN_GLOBAL_DIR"
fi

echo "vyn uninstalled."
