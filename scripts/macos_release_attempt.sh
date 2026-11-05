#!/bin/bash

record_failed_attempt_and_cleanup() {
  local status=$?
  if (( status != 0 )) && [[ -n "${EVIDENCE_DIR:-}" && -d "$EVIDENCE_DIR" ]]; then
    python3 "$ROOT/tools/record_macos_release_failure.py" \
      "$EVIDENCE_DIR/release-context.json" "$SOURCE_SHA" "$status" || true
  fi
  rm -rf "$TMP" "${OUTPUT_STAGE:-}"
  return "$status"
}
