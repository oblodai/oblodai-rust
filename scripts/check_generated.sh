#!/usr/bin/env bash
# Fail when src/generated is not what the generator makes of the gateway's contract.
#
# Regenerates into a temporary directory with the backend's tools/sdkgen (from
# services/core/api/openapi.json, checked against names.lock) and compares file by file. The
# backend checkout is $OBLODAI_BACKEND, else ../oblodai-backend next to this repository. Without a
# backend that has tools/sdkgen the check is skipped, loudly; with --require it fails instead.
# Fix drift by regenerating (`make sdk` in the backend), never by hand.
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
backend="${OBLODAI_BACKEND:-$root/../oblodai-backend}"
sdkgen="$backend/tools/sdkgen"
spec="$backend/services/core/api/openapi.json"

if [ ! -d "$sdkgen/cmd/sdkgen" ] || [ ! -f "$spec" ]; then
  msg="no generator at $sdkgen (set OBLODAI_BACKEND to the backend checkout)"
  if [ "${1:-}" = "--require" ]; then
    echo "check_generated: $msg" >&2
    exit 1
  fi
  echo "  (skipped: $msg)"
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
if ! (cd "$sdkgen" && GOTOOLCHAIN="${GOTOOLCHAIN:-go1.26.6}" go run ./cmd/sdkgen \
  -spec "$spec" -lang rust -out "$tmp/out" -lock "$root/names.lock") >"$tmp/log" 2>&1; then
  echo "check_generated: sdkgen failed:" >&2
  cat "$tmp/log" >&2
  exit 1
fi
if ! diff -r "$tmp/out/src/generated" "$root/src/generated" >/dev/null; then
  echo "check_generated: src/generated is stale:" >&2
  diff -rq "$tmp/out/src/generated" "$root/src/generated" >&2 || true
  echo "regenerate with \`make sdk\` in the backend" >&2
  exit 1
fi
echo "generated code matches $spec"
