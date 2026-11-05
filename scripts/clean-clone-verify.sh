#!/usr/bin/env bash
set -eu

SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
SOURCE_ROOT="$(cd "$(dirname "$SCRIPT_PATH")/.." 2>/dev/null && pwd -P)" || SOURCE_ROOT=""

if [ -z "$SOURCE_ROOT" ] || [ "$SOURCE_ROOT" = "/" ] || [ ! -f "$SOURCE_ROOT/Cargo.toml" ]; then
  printf 'clean-clone-verify: cannot locate the repository root\n' >&2
  exit 1
fi

CLONE_ROOT="$SOURCE_ROOT/artifacts/tmp/clean-clone"
rm -rf "$CLONE_ROOT"
mkdir -p "$SOURCE_ROOT/artifacts/tmp"
git -C "$SOURCE_ROOT" clone --quiet --no-local "$SOURCE_ROOT" "$CLONE_ROOT"
git -C "$CLONE_ROOT" checkout --quiet HEAD

ln -sfn "$SOURCE_ROOT/.rustup-home" "$CLONE_ROOT/.rustup-home"
ln -sfn "$SOURCE_ROOT/.cargo-home" "$CLONE_ROOT/.cargo-home"
ln -sfn "$SOURCE_ROOT/target" "$CLONE_ROOT/target"
ln -sfn "$SOURCE_ROOT/.tools" "$CLONE_ROOT/.tools"
mkdir -p "$CLONE_ROOT/artifacts"

ORIGINAL_PATH="$PATH"
export CARGO_HOME="$SOURCE_ROOT/.cargo-home"
export RUSTUP_HOME="$SOURCE_ROOT/.rustup-home"
export CARGO_TARGET_DIR="$SOURCE_ROOT/target"
export TMPDIR="$SOURCE_ROOT/.tmp"
export TMP="$TMPDIR"
export TEMP="$TMPDIR"
export XDG_CACHE_HOME="$SOURCE_ROOT/.cache"
export PATH="$SOURCE_ROOT/.tools/bin:$CARGO_HOME/bin:$ORIGINAL_PATH"

cd "$CLONE_ROOT"
just verify
