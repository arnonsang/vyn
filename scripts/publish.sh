#!/usr/bin/env bash
# publish.sh - Publish all vyn crates to crates.io in dependency order.
#
# Publish order is mandatory:
#   1. vyn-relay  (no local deps)
#   2. vyn-core   (depends on vyn-relay)
#   3. vyn-cli    (depends on vyn-core + vyn-relay)
#
# Usage:
#   ./scripts/publish.sh             # real publish
#   ./scripts/publish.sh --dry-run   # dry run (no upload, validates packaging)
#
# Requirements:
#   - cargo login <token>  must be done beforehand
#   - Run from the workspace root

set -euo pipefail

CRATES=(vyn-relay vyn-core vyn-cli)
DRY_RUN=false
DELAY=40   # seconds to wait between publishes for crates.io index to update

# ── Argument parsing ──────────────────────────────────────────────────────────
for arg in "$@"; do
  case "$arg" in
    --dry-run|-n)
      DRY_RUN=true
      ;;
    --help|-h)
      sed -n '/^# /s/^# //p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      echo "Usage: $0 [--dry-run]" >&2
      exit 1
      ;;
  esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────
log()  { echo "  [vyn-publish] $*"; }
ok()   { echo "  ✔ $*"; }
step() { echo; echo "──────────────────────────────────────"; echo "  $*"; echo "──────────────────────────────────────"; }

# ── Preflight ─────────────────────────────────────────────────────────────────
step "Preflight"

# Must be run from workspace root
if [[ ! -f Cargo.toml ]] || ! grep -q '^\[workspace\]' Cargo.toml; then
  echo "Error: run this script from the workspace root (where Cargo.toml has [workspace])" >&2
  exit 1
fi

# Ensure cargo is available
if ! command -v cargo &>/dev/null; then
  echo "Error: cargo not found on PATH" >&2
  exit 1
fi

if $DRY_RUN; then
  log "Mode: DRY RUN: packages will be validated but not uploaded"
else
  log "Mode: REAL PUBLISH: packages will be uploaded to crates.io"
  log "Crates.io token must already be configured (cargo login <token>)"
fi

log "Publish order: ${CRATES[*]}"
log "Reason: each crate depends on the previous one; crates.io requires"
log "        the dependency to be indexed before the dependent is uploaded."

# ── Build check ───────────────────────────────────────────────────────────────
step "Build check (release)"
cargo build --release --workspace
ok "workspace builds cleanly"

# ── Test check ────────────────────────────────────────────────────────────────
step "Tests"
cargo test --workspace
ok "all tests pass"

# ── Publish loop ─────────────────────────────────────────────────────────────
step "Publishing"

for i in "${!CRATES[@]}"; do
  crate="${CRATES[$i]}"
  step "[$((i+1))/${#CRATES[@]}] $crate"

  if $DRY_RUN; then
    if [[ $i -eq 0 ]]; then
      # First crate has no local deps — full publish dry-run works fine.
      log "dry-run: cargo publish --dry-run --allow-dirty -p $crate"
      cargo publish --dry-run --allow-dirty -p "$crate"
      ok "$crate: dry run passed"
    else
      # Dependent crates can't be dry-run-verified until the preceding crate is
      # indexed on crates.io. The workspace build check above already validates
      # compilation for these crates.
      log "skipping publish dry-run for $crate (depends on local crates not yet indexed; workspace build already validated)"
      ok "$crate: skipped (workspace build passed)"
    fi
  else
    log "publishing $crate…"
    cargo publish --allow-dirty -p "$crate"
    ok "$crate: published"

    # Wait for crates.io index to update before publishing the next crate,
    # unless this is the last one.
    if [[ $((i+1)) -lt ${#CRATES[@]} ]]; then
      log "Waiting ${DELAY}s for crates.io index to propagate before next crate…"
      sleep "$DELAY"
    fi
  fi
done

# ── Done ──────────────────────────────────────────────────────────────────────
echo
if $DRY_RUN; then
  ok "Dry run complete: all crates passed validation."
  echo "  Run without --dry-run to publish for real."
else
  ok "All crates published successfully."
  echo "  View them at: https://crates.io/crates/vyn-cli"
fi
echo
