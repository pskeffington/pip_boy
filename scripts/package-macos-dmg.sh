#!/bin/bash
set -euo pipefail
umask 077
if [[ "$(uname -s)" != Darwin ]]; then echo "This packager requires macOS." >&2; exit 1; fi
if [[ -z "${RUSTBOY_SIGN_IDENTITY:-}" || -z "${RUSTBOY_NOTARY_PROFILE:-}" ]]; then
  echo "Set RUSTBOY_SIGN_IDENTITY and RUSTBOY_NOTARY_PROFILE; unsigned distribution is refused." >&2
  exit 2
fi
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
SOURCE_SHA="$(git -C "$ROOT" rev-parse HEAD)"
if [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
  echo "Refusing release packaging from a dirty working tree; commit or discard changes first." >&2
  exit 2
fi
APP="${RUSTBOY_APP_PATH:-$ROOT/target/macos/Rust-Boy.app}"
OUTPUT="${RUSTBOY_DMG_OUTPUT:-$ROOT/target/macos/Rust-Boy.dmg}"
OUTPUT_NAME="$(basename -- "$OUTPUT")"
OUTPUT_PARENT_RAW="$(dirname -- "$OUTPUT")"
mkdir -p "$OUTPUT_PARENT_RAW"
OUTPUT_PARENT="$(cd -- "$OUTPUT_PARENT_RAW" && pwd -P)"
if [[ "$OUTPUT_NAME" != *.dmg || "$OUTPUT_PARENT" == "/" || -L "$OUTPUT_PARENT/$OUTPUT_NAME" || -d "$OUTPUT_PARENT/$OUTPUT_NAME" ]]; then
  printf 'Unsafe DMG output path: %s\n' "$OUTPUT" >&2
  exit 2
fi
OUTPUT="$OUTPUT_PARENT/$OUTPUT_NAME"
EVIDENCE_ROOT="${RUSTBOY_RELEASE_EVIDENCE_DIR:-$ROOT/target/macos/release-evidence}"
mkdir -p "$EVIDENCE_ROOT"
[[ ! -L "$EVIDENCE_ROOT" ]] || { printf 'Refusing symlink evidence root: %s\n' "$EVIDENCE_ROOT" >&2; exit 2; }
EVIDENCE_ROOT="$(cd -- "$EVIDENCE_ROOT" && pwd -P)"
if [[ "$EVIDENCE_ROOT" == "/" || "$EVIDENCE_ROOT" == "$APP" || "$EVIDENCE_ROOT" == "$OUTPUT_PARENT" ]]; then
  printf 'Unsafe release evidence root: %s\n' "$EVIDENCE_ROOT" >&2
  exit 2
fi
python3 - "$EVIDENCE_ROOT" "$APP" <<'PY'
import os
import sys
from pathlib import Path

evidence_root = Path(sys.argv[1]).resolve()
app_bundle = Path(sys.argv[2]).resolve()
if os.path.commonpath((str(evidence_root), str(app_bundle))) == str(app_bundle):
    print("Release evidence must be stored outside the signed app bundle.", file=sys.stderr)
    raise SystemExit(2)
PY
EVIDENCE_DIR="$EVIDENCE_ROOT/$SOURCE_SHA"
[[ ! -L "$EVIDENCE_DIR" ]] || { printf 'Refusing symlink evidence directory: %s\n' "$EVIDENCE_DIR" >&2; exit 2; }
mkdir -p "$EVIDENCE_DIR"
chmod 700 "$EVIDENCE_DIR"
TMP="$(mktemp -d)"
OUTPUT_STAGE="$(mktemp -d "$OUTPUT_PARENT/.rustboy-dmg.XXXXXX")"
trap 'rm -rf "$TMP" "$OUTPUT_STAGE"' EXIT
STAGE="$TMP/stage"
DMG_CANDIDATE="$OUTPUT_STAGE/Rust-Boy.dmg"
mkdir -p "$STAGE"
[[ -d "$APP" ]] || { printf 'Missing app bundle: %s\n' "$APP" >&2; exit 1; }
APP_ARCHS="$(lipo -archs "$APP/Contents/MacOS/RustBoy")"
SERVICE_ARCHS="$(lipo -archs "$APP/Contents/MacOS/pipboy")"
for architecture in arm64 x86_64; do
  [[ " $APP_ARCHS " == *" $architecture "* && " $SERVICE_ARCHS " == *" $architecture "* ]] || {
    printf 'App and service must both contain %s slices (host=%s service=%s).\n' "$architecture" "$APP_ARCHS" "$SERVICE_ARCHS" >&2
    exit 2
  }
done
APP_BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist")"
[[ "$APP_BUNDLE_ID" == org.rustboy.desktop ]] || { printf 'Unexpected app bundle identifier: %s\n' "$APP_BUNDLE_ID" >&2; exit 2; }
APP_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
python3 - "$EVIDENCE_DIR/release-context.json" "$SOURCE_SHA" "$APP_VERSION" "$APP_BUNDLE_ID" "$APP_ARCHS" "$OUTPUT" <<'PY'
import json
import sys
from datetime import datetime, timezone

path, source_sha, version, bundle_id, architecture, artifact = sys.argv[1:]
with open(path, "w", encoding="utf-8") as output:
    json.dump(
        {
            "schema_version": 1,
            "state": "in_progress",
            "started_utc": datetime.now(timezone.utc).isoformat(),
            "source_sha": source_sha,
            "version": version,
            "bundle_identifier": bundle_id,
            "architecture": architecture,
            "artifact": artifact,
        },
        output,
        indent=2,
        sort_keys=True,
    )
    output.write("\n")
PY
codesign --verify --deep --strict --verbose=2 "$APP"
codesign -dv --verbose=4 "$APP" > "$EVIDENCE_DIR/app-codesign.txt" 2>&1
grep -q '^Authority=Developer ID Application:' "$EVIDENCE_DIR/app-codesign.txt" || {
  echo "The app must be signed with a Developer ID Application identity." >&2
  exit 2
}
ditto -c -k --keepParent "$APP" "$TMP/Rust-Boy.zip"
APP_SUBMISSION_JSON="$EVIDENCE_DIR/app-notary-submission.json"
xcrun notarytool submit "$TMP/Rust-Boy.zip" --keychain-profile "$RUSTBOY_NOTARY_PROFILE" --wait --output-format json > "$APP_SUBMISSION_JSON"
APP_SUBMISSION_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["id"])' "$APP_SUBMISSION_JSON")"
APP_SUBMISSION_STATUS="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["status"])' "$APP_SUBMISSION_JSON")"
xcrun notarytool log "$APP_SUBMISSION_ID" --keychain-profile "$RUSTBOY_NOTARY_PROFILE" "$EVIDENCE_DIR/app-notary-log.json"
[[ "$APP_SUBMISSION_STATUS" == "Accepted" ]] || { printf 'App notarization was not accepted: %s (evidence: %s)\n' "$APP_SUBMISSION_STATUS" "$EVIDENCE_DIR" >&2; exit 1; }
xcrun stapler staple "$APP"
xcrun stapler validate "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
spctl --assess --type execute --verbose=2 "$APP"
ditto "$APP" "$STAGE/Rust-Boy.app"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Rust-Boy" -srcfolder "$STAGE" -ov -format UDZO "$DMG_CANDIDATE" >/dev/null
codesign --force --timestamp --sign "$RUSTBOY_SIGN_IDENTITY" "$DMG_CANDIDATE"
codesign --verify --strict --verbose=2 "$DMG_CANDIDATE"
codesign -dv --verbose=4 "$DMG_CANDIDATE" > "$EVIDENCE_DIR/dmg-codesign.txt" 2>&1
grep -q '^Authority=Developer ID Application:' "$EVIDENCE_DIR/dmg-codesign.txt" || {
  echo "The disk image must be signed with a Developer ID Application identity." >&2
  exit 2
}
DMG_SUBMISSION_JSON="$EVIDENCE_DIR/dmg-notary-submission.json"
xcrun notarytool submit "$DMG_CANDIDATE" --keychain-profile "$RUSTBOY_NOTARY_PROFILE" --wait --output-format json > "$DMG_SUBMISSION_JSON"
DMG_SUBMISSION_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["id"])' "$DMG_SUBMISSION_JSON")"
DMG_SUBMISSION_STATUS="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["status"])' "$DMG_SUBMISSION_JSON")"
xcrun notarytool log "$DMG_SUBMISSION_ID" --keychain-profile "$RUSTBOY_NOTARY_PROFILE" "$EVIDENCE_DIR/dmg-notary-log.json"
[[ "$DMG_SUBMISSION_STATUS" == "Accepted" ]] || { printf 'DMG notarization was not accepted: %s (evidence: %s)\n' "$DMG_SUBMISSION_STATUS" "$EVIDENCE_DIR" >&2; exit 1; }
xcrun stapler staple "$DMG_CANDIDATE"
xcrun stapler validate "$DMG_CANDIDATE"
spctl --assess --type open --context context:primary-signature --verbose=2 "$DMG_CANDIDATE"
BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist")"
APP_ZIP_SHA256="$(shasum -a 256 "$TMP/Rust-Boy.zip" | awk '{print $1}')"
APP_SHA256="$(shasum -a 256 "$APP/Contents/MacOS/RustBoy" | awk '{print $1}')"
SERVICE_SHA256="$(shasum -a 256 "$APP/Contents/MacOS/pipboy" | awk '{print $1}')"
DMG_SHA256="$(shasum -a 256 "$DMG_CANDIDATE" | awk '{print $1}')"
python3 - "$EVIDENCE_DIR/release-evidence.json" "$SOURCE_SHA" "$APP_VERSION" "$BUNDLE_ID" "$APP_SUBMISSION_ID" "$DMG_SUBMISSION_ID" "$APP_ZIP_SHA256" "$APP_SHA256" "$SERVICE_SHA256" "$DMG_SHA256" "$OUTPUT" "$APP_ARCHS" <<'PY'
import json
import sys
from datetime import datetime, timezone

(
    path, source_sha, app_version, bundle_id, app_submission_id,
    dmg_submission_id, app_zip_sha256, app_sha256, service_sha256, dmg_sha256, dmg_path, architecture,
) = sys.argv[1:]
record = {
    "schema_version": 1,
    "created_utc": datetime.now(timezone.utc).isoformat(),
    "source_sha": source_sha,
    "bundle_identifier": bundle_id,
    "version": app_version,
    "architecture": architecture,
    "notarization": {
        "app": {"submission_id": app_submission_id, "status": "Accepted", "log": "app-notary-log.json"},
        "dmg": {"submission_id": dmg_submission_id, "status": "Accepted", "log": "dmg-notary-log.json"},
    },
    "sha256": {
        "app_rustboy_executable": app_sha256,
        "bundled_pipboy_service": service_sha256,
        "app_notarization_zip": app_zip_sha256,
        "dmg": dmg_sha256,
    },
    "artifact": dmg_path,
}
with open(path, "w", encoding="utf-8") as output:
    json.dump(record, output, indent=2, sort_keys=True)
    output.write("\n")
PY
cp "$TMP/Rust-Boy.zip" "$EVIDENCE_DIR/Rust-Boy.app.zip"
mv -f "$DMG_CANDIDATE" "$OUTPUT"
python3 - "$EVIDENCE_DIR/release-context.json" "$APP_SUBMISSION_ID" "$DMG_SUBMISSION_ID" <<'PY'
import json
import sys
from datetime import datetime, timezone

path, app_submission_id, dmg_submission_id = sys.argv[1:]
with open(path, encoding="utf-8") as source:
    record = json.load(source)
record["state"] = "completed"
record["completed_utc"] = datetime.now(timezone.utc).isoformat()
record["notarization_submission_ids"] = {
    "app": app_submission_id,
    "dmg": dmg_submission_id,
}
with open(path, "w", encoding="utf-8") as output:
    json.dump(record, output, indent=2, sort_keys=True)
    output.write("\n")
PY
printf 'Created and notarized %s\nRelease evidence: %s\n' "$OUTPUT" "$EVIDENCE_DIR"
