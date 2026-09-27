#!/usr/bin/env bash
# Vendors the contract snapshot the shared conformance suite needs, so CI runs it without the private
# backend checkout: the suite's scenario files (backend tools/sdkgen/conformance) and the
# `x-oblodai-signing` section of services/core/api/openapi.json, in the same relative layout under
# contract/ (SDKGEN_CONFORMANCE=contract/tools/sdkgen/conformance).
#
#   ./scripts/vendor_contract.sh           refresh contract/ from $OBLODAI_BACKEND (else ../oblodai-backend)
#   ./scripts/vendor_contract.sh --check   fail when contract/ differs from the backend (skipped without one)
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
backend="${OBLODAI_BACKEND:-$root/../oblodai-backend}"
suite="$backend/tools/sdkgen/conformance"
spec="$backend/services/core/api/openapi.json"
if [ ! -d "$suite" ] || [ ! -f "$spec" ]; then
  echo "  (skipped: no conformance suite at $suite; set OBLODAI_BACKEND)"
  exit 0
fi
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/contract/tools/sdkgen/conformance" "$tmp/contract/services/core/api"
cp "$suite"/*.json "$tmp/contract/tools/sdkgen/conformance/"
python3 - "$spec" "$tmp/contract/services/core/api/openapi.json" <<'PY'
import json, sys
spec = json.load(open(sys.argv[1], encoding="utf-8"))
out = {"x-oblodai-signing": spec["x-oblodai-signing"]}
with open(sys.argv[2], "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=1, sort_keys=True)
    f.write("\n")
PY
if [ "${1:-}" = "--check" ]; then
  if ! diff -r "$tmp/contract" "$root/contract" >/dev/null 2>&1; then
    diff -r "$tmp/contract" "$root/contract" | head -20 >&2 || true
    echo "vendor_contract: contract/ is stale; run ./scripts/vendor_contract.sh" >&2
    exit 1
  fi
  echo "  contract snapshot matches the backend"
  exit 0
fi
rm -rf "$root/contract"
cp -r "$tmp/contract" "$root/contract"
echo "vendored the conformance suite and x-oblodai-signing into contract/"
