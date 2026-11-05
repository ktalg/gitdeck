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

SCREENSHOT_ROOT="$REPO_ROOT/artifacts/screenshots"
MANIFEST_PATH="$SCREENSHOT_ROOT/manifest.txt"
STAMP_PATH="$REPO_ROOT/.tmp/suite-run-stamp"
DIFF_SCREENSHOT_ROOT="$SCREENSHOT_ROOT/diff"
FAILURE_COUNT=0

report_failure() {
  printf 'FAIL %s\n' "$1"
  FAILURE_COUNT=$((FAILURE_COUNT + 1))
}

report_gate_result() {
  if [ "$FAILURE_COUNT" -ne 0 ]; then
    printf 'screenshot gate failed: %s problem(s)\n' "$FAILURE_COUNT"
    exit 1
  fi
}

sha256_of_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

if [ ! -f "$MANIFEST_PATH" ]; then
  report_failure "screenshot manifest ${MANIFEST_PATH#"$REPO_ROOT"/} is missing; run just test-suite first"
  report_gate_result
  exit 1
fi

if [ ! -s "$MANIFEST_PATH" ]; then
  report_failure "screenshot manifest ${MANIFEST_PATH#"$REPO_ROOT"/} lists no screenshots"
  report_gate_result
  exit 1
fi

if [ ! -f "$STAMP_PATH" ]; then
  report_failure "suite run stamp ${STAMP_PATH#"$REPO_ROOT"/} is missing; run just test-suite first"
  report_gate_result
  exit 1
fi

LISTED_ARTIFACT_PATHS=""

while IFS= read -r MANIFEST_LINE; do
  if [ -z "$MANIFEST_LINE" ]; then
    continue
  fi
  EXPECTED_SHA256="${MANIFEST_LINE%%  *}"
  RELATIVE_PATH="${MANIFEST_LINE#*  }"
  if [ "$RELATIVE_PATH" = "$MANIFEST_LINE" ] || [ -z "$RELATIVE_PATH" ]; then
    report_failure "manifest line is not '<sha256>  <path>': $MANIFEST_LINE"
    continue
  fi
  LISTED_ARTIFACT_PATHS="$LISTED_ARTIFACT_PATHS$RELATIVE_PATH
"
  ARTIFACT_PATH="$REPO_ROOT/$RELATIVE_PATH"
  if [ ! -f "$ARTIFACT_PATH" ]; then
    report_failure "listed artifact $RELATIVE_PATH is missing"
    continue
  fi
  ACTUAL_SHA256="$(sha256_of_file "$ARTIFACT_PATH")"
  if [ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]; then
    report_failure "$RELATIVE_PATH has sha256 $ACTUAL_SHA256 but the manifest lists $EXPECTED_SHA256"
  fi
  if [ -z "$(find "$ARTIFACT_PATH" -newer "$STAMP_PATH" -print)" ]; then
    report_failure "$RELATIVE_PATH is stale: its mtime is not newer than ${STAMP_PATH#"$REPO_ROOT"/}"
  fi
done < "$MANIFEST_PATH"

while IFS= read -r SCREENSHOT_PATH; do
  if [ -z "$SCREENSHOT_PATH" ]; then
    continue
  fi
  RELATIVE_PATH="${SCREENSHOT_PATH#"$REPO_ROOT"/}"
  if ! printf '%s' "$LISTED_ARTIFACT_PATHS" | grep -Fxq "$RELATIVE_PATH"; then
    report_failure "screenshot $RELATIVE_PATH exists on disk but is missing from the manifest"
  fi
done <<< "$(find "$SCREENSHOT_ROOT" -type f -name '*.png' -not -path "$DIFF_SCREENSHOT_ROOT/*" | sort)"

report_gate_result

UNIQUE_ARTIFACT_PATHS="$(printf '%s' "$LISTED_ARTIFACT_PATHS" | sed '/^$/d' | LC_ALL=C sort -u)"
printf '%s\n' "$UNIQUE_ARTIFACT_PATHS"
COLLECTED_SCREENSHOT_COUNT="$(printf '%s\n' "$UNIQUE_ARTIFACT_PATHS" | grep -c '\.png$')"
printf 'collected %s screenshot(s)\n' "$COLLECTED_SCREENSHOT_COUNT"
