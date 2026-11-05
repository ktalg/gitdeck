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

CHECK_FAILURES=0

REDIRECTED_VARIABLES=(
  CARGO_HOME
  RUSTUP_HOME
  CARGO_TARGET_DIR
  TMPDIR
  TMP
  TEMP
  XDG_CACHE_HOME
  XDG_DATA_HOME
  XDG_CONFIG_HOME
  GIT_CONFIG_GLOBAL
  GIT_CONFIG_SYSTEM
  HOME
)

print_check_failure() {
  printf 'FAIL %s\n' "$1"
  CHECK_FAILURES=$((CHECK_FAILURES + 1))
}

print_check_pass() {
  printf 'ok   %s\n' "$1"
}

assert_variable_resolves_inside_repo() {
  local variable_name="$1"
  local variable_value="${!variable_name:-}"
  if [ -z "$variable_value" ]; then
    print_check_failure "$variable_name is unset or empty"
    return
  fi
  case "$variable_value" in
    "$REPO_ROOT"/*) print_check_pass "$variable_name -> $variable_value" ;;
    *) print_check_failure "$variable_name -> $variable_value escapes $REPO_ROOT" ;;
  esac
}

assert_git_is_available() {
  local git_version
  if ! git_version="$(git --version 2>&1)"; then
    print_check_failure "git --version failed ($git_version): test fixture repositories cannot be generated"
    return
  fi
  print_check_pass "git available -> $git_version"
}

assert_git_config_nosystem_enabled() {
  if [ "${GIT_CONFIG_NOSYSTEM:-}" = "1" ]; then
    print_check_pass "GIT_CONFIG_NOSYSTEM -> 1"
  else
    print_check_failure "GIT_CONFIG_NOSYSTEM -> ${GIT_CONFIG_NOSYSTEM:-unset} (expected 1)"
  fi
}

assert_named_paths_absent_outside_repo() {
  local real_user_home
  real_user_home="$(eval echo "~$(id -un)")"
  local forbidden_path
  for forbidden_path in \
    "$real_user_home/.gitdeck" \
    "$real_user_home/.config/gitdeck" \
    "$real_user_home/.cache/gitdeck" \
    "$real_user_home/Library/Application Support/gitdeck"; do
    if [ -e "$forbidden_path" ]; then
      print_check_failure "unexpected path outside repo -> $forbidden_path"
    else
      print_check_pass "absent $forbidden_path"
    fi
  done
  local temp_candidate
  for temp_candidate in /tmp/gitdeck* /var/tmp/gitdeck*; do
    if [ -e "$temp_candidate" ]; then
      print_check_failure "unexpected path outside repo -> $temp_candidate"
    else
      print_check_pass "absent $temp_candidate"
    fi
  done
}

collect_own_process_chain() {
  local process_id="$$"
  while [ -n "$process_id" ] && [ "$process_id" -gt 1 ]; do
    printf '%s\n' "$process_id"
    process_id="$(ps -o ppid= -p "$process_id" 2>/dev/null | tr -d ' ')"
  done
}

assert_no_residual_gitdeck_process() {
  local own_process_chain
  own_process_chain="$(collect_own_process_chain)"
  local residual_processes
  residual_processes="$(pgrep -f gitdeck || true)"
  local residual_process_count=0
  local process_id
  for process_id in $residual_processes; do
    if printf '%s\n' "$own_process_chain" | grep -qx "$process_id"; then
      continue
    fi
    print_check_failure "residual process $process_id -> $(ps -o command= -p "$process_id" 2>/dev/null)"
    residual_process_count=$((residual_process_count + 1))
  done
  if [ "$residual_process_count" -eq 0 ]; then
    print_check_pass "no residual gitdeck process"
  fi
}

assert_redirected_variables_resolve_inside_repo() {
  local variable_name
  for variable_name in "${REDIRECTED_VARIABLES[@]}"; do
    assert_variable_resolves_inside_repo "$variable_name"
  done
}

assert_shell_portability() {
  local portability_output portability_status
  portability_output="$(bash "$REPO_ROOT/scripts/shell-portability-test.sh" 2>&1)"
  portability_status=$?
  if [ "$portability_status" -eq 0 ]; then
    print_check_pass "shell portability -> $(printf '%s' "$portability_output" | tail -n 1)"
    return
  fi
  print_check_failure "shell portability test failed (status $portability_status)"
  printf '%s\n' "$portability_output"
}

assert_redirected_variables_resolve_inside_repo
assert_git_config_nosystem_enabled
assert_git_is_available
assert_named_paths_absent_outside_repo
assert_no_residual_gitdeck_process
assert_shell_portability

if [ "$CHECK_FAILURES" -ne 0 ]; then
  printf 'sandbox check failed: %s problem(s)\n' "$CHECK_FAILURES"
  exit 1
fi

printf 'sandbox check passed\n'
