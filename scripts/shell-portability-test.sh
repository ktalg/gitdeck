#!/usr/bin/env bash
set -u

REPO_SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
REPO_ROOT=""
if [ -n "$REPO_SCRIPT_PATH" ]; then
  REPO_ROOT="$(cd "$(dirname "$REPO_SCRIPT_PATH")/.." 2>/dev/null && pwd -P)" || REPO_ROOT=""
fi

if [ -z "$REPO_ROOT" ] || [ "$REPO_ROOT" = "/" ] || [ ! -f "$REPO_ROOT/Cargo.toml" ] || [ ! -d "$REPO_ROOT/crates" ]; then
  printf 'cannot locate gitdeck repository root from %s\n' "${REPO_SCRIPT_PATH:-<empty>}" >&2
  exit 1
fi

PORTABILITY_PROBE_DIRECTORY="$REPO_ROOT/.tmp/portability"
FAILURE_COUNT=0

report_pass() {
  printf 'ok   %s\n' "$1"
}

report_failure() {
  printf 'FAIL %s\n' "$1"
  FAILURE_COUNT=$((FAILURE_COUNT + 1))
}

assert_equal() {
  local label="$1"
  local expected_value="$2"
  local actual_value="$3"
  if [ "$expected_value" = "$actual_value" ]; then
    report_pass "$label -> $actual_value"
  else
    report_failure "$label -> expected $expected_value, actual $actual_value"
  fi
}

assert_path_inside_repository() {
  local label="$1"
  local path_value="$2"
  case "$path_value" in
    "$REPO_ROOT"/*) report_pass "$label -> $path_value" ;;
    *) report_failure "$label -> $path_value escapes $REPO_ROOT" ;;
  esac
}

print_repository_root_from_repository_root() {
  local shell_name="$1"
  ( cd "$REPO_ROOT" && "$shell_name" -c 'source scripts/sandbox-env.sh && printf %s "$REPO_ROOT"' )
}

print_repository_root_from_crates_directory() {
  local shell_name="$1"
  ( cd "$REPO_ROOT/crates" && "$shell_name" -c 'source ../scripts/sandbox-env.sh && printf %s "$REPO_ROOT"' )
}

print_redirected_variable() {
  local shell_name="$1"
  local variable_name="$2"
  ( cd "$REPO_ROOT" && "$shell_name" -c "source scripts/sandbox-env.sh && printf %s \"\$$variable_name\"" )
}

assert_missing_repository_root_fails_loudly() {
  local shell_name="$1"
  mkdir -p "$PORTABILITY_PROBE_DIRECTORY"
  cp "$REPO_ROOT/scripts/sandbox-env.sh" "$PORTABILITY_PROBE_DIRECTORY/sandbox-env.sh"
  local reported_output reported_status
  reported_output="$( cd / && "$shell_name" -c "source $PORTABILITY_PROBE_DIRECTORY/sandbox-env.sh" 2>&1 )"
  reported_status=$?
  if [ "$reported_status" -ne 0 ]; then
    report_pass "$shell_name unreachable repository root exits with status $reported_status"
  else
    report_failure "$shell_name unreachable repository root exits with status 0"
  fi
  case "$reported_output" in
    *'cannot locate gitdeck repository root from'*) report_pass "$shell_name unreachable repository root reports the failure" ;;
    *) report_failure "$shell_name unreachable repository root printed [$reported_output]" ;;
  esac
  local leaked_directory
  for leaked_directory in "$REPO_ROOT/.tmp/.cargo-home" "$REPO_ROOT/.tmp/.cache" "$REPO_ROOT/.tmp/.home" "$REPO_ROOT/.tmp/artifacts"; do
    if [ -e "$leaked_directory" ]; then
      report_failure "$shell_name unreachable repository root created ${leaked_directory#"$REPO_ROOT"/}"
    else
      report_pass "$shell_name created no ${leaked_directory#"$REPO_ROOT"/}"
    fi
  done
  rm -f "$PORTABILITY_PROBE_DIRECTORY/sandbox-env.sh"
}

run_shell_portability_checks() {
  local shell_name="$1"
  assert_equal "$shell_name repository root from repository root" "$REPO_ROOT" "$(print_repository_root_from_repository_root "$shell_name")"
  assert_equal "$shell_name repository root from crates directory" "$REPO_ROOT" "$(print_repository_root_from_crates_directory "$shell_name")"
  assert_path_inside_repository "$shell_name HOME" "$(print_redirected_variable "$shell_name" HOME)"
  assert_path_inside_repository "$shell_name TMPDIR" "$(print_redirected_variable "$shell_name" TMPDIR)"
  assert_missing_repository_root_fails_loudly "$shell_name"
}

run_shell_portability_checks bash

if command -v zsh >/dev/null 2>&1; then
  run_shell_portability_checks zsh
else
  printf 'skipped zsh\n'
fi

if [ "$FAILURE_COUNT" -ne 0 ]; then
  printf 'shell portability test failed: %s problem(s)\n' "$FAILURE_COUNT"
  exit 1
fi

printf 'shell portability test passed\n'
