#!/usr/bin/env bash
set -u

REPO_SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
REPO_ROOT=""
if [ -n "$REPO_SCRIPT_PATH" ]; then
  REPO_ROOT="$(cd "$(dirname "$REPO_SCRIPT_PATH")/.." 2>/dev/null && pwd -P)" || REPO_ROOT=""
fi

if [ -z "$REPO_ROOT" ] || [ "$REPO_ROOT" = "/" ] || [ ! -f "$REPO_ROOT/Cargo.toml" ] || [ ! -d "$REPO_ROOT/crates" ]; then
  printf 'cannot locate gitdeck repository root from %s\n' "${REPO_SCRIPT_PATH:-<empty>}" >&2
  if [ -n "${ZSH_EVAL_CONTEXT:-}" ]; then
    case "$ZSH_EVAL_CONTEXT" in
      file|*:file) return 1 ;;
    esac
  fi
  if [ -n "${BASH_SOURCE[0]:-}" ] && [ "${BASH_SOURCE[0]}" != "$0" ]; then
    return 1
  fi
  exit 1
fi

export CARGO_HOME="$REPO_ROOT/.cargo-home"
export RUSTUP_HOME="$REPO_ROOT/.rustup-home"
export CARGO_TARGET_DIR="$REPO_ROOT/target"
export TMPDIR="$REPO_ROOT/.tmp"
export TMP="$TMPDIR"
export TEMP="$TMPDIR"
export XDG_CACHE_HOME="$REPO_ROOT/.cache"
export XDG_DATA_HOME="$REPO_ROOT/.cache/data"
export XDG_CONFIG_HOME="$REPO_ROOT/.cache/config"
export GIT_CONFIG_GLOBAL="$REPO_ROOT/.cache/gitconfig"
export GIT_CONFIG_SYSTEM="$REPO_ROOT/.cache/gitconfig-system"
export GIT_CONFIG_NOSYSTEM=1
export HOME="$REPO_ROOT/.home"
export PATH="$REPO_ROOT/.tools/bin:$CARGO_HOME/bin:$PATH"

mkdir -p "$TMPDIR" "$XDG_CACHE_HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$REPO_ROOT/.home" "$REPO_ROOT/artifacts"
