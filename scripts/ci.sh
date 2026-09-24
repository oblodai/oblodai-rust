#!/usr/bin/env bash
# Every gate the CI runs, in the order that fails fastest. Usage: ./scripts/ci.sh [--live]
#
# Builds run with two jobs (CARGO_BUILD_JOBS, default 2) and the cache stays in ./target
# (git-ignored). The drift and conformance gates read the backend checkout: $OBLODAI_BACKEND, else
# ../oblodai-backend next to this repository.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-never}"

echo "== generated code drift"
./scripts/check_generated.sh

echo "== format"
cargo fmt --all --check

echo "== lints (every feature combination that ships)"
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo clippy --locked --all-targets --no-default-features --features reqwest-client -- -D warnings

echo "== unit + conformance + examples + README tests"
cargo test --locked --all-features

echo "== docs"
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features

echo "== package"
files="$(cargo package --locked --allow-dirty --list)"
for path in src/generated/resources.rs src/lro.rs README.md CHANGELOG.md MIGRATION-2.0.md AGENTS.md \
  names.lock LICENSE; do
  grep -qx "$path" <<<"$files" || { echo "the crate is missing $path" >&2; exit 1; }
done
cargo package --locked --allow-dirty --no-verify --quiet
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
echo "package: target/package/oblodai-$version.crate"

echo "== MSRV"
msrv="$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)"
if rustup run "$msrv" cargo --version >/dev/null 2>&1; then
  RUSTUP_TOOLCHAIN="$msrv" cargo check --locked --all-features
  RUSTUP_TOOLCHAIN="$msrv" cargo check --locked --no-default-features
else
  echo "  (skipped: toolchain $msrv is not installed; rustup toolchain install $msrv)"
fi

if [ "${1:-}" = "--live" ]; then
  echo "== live tests against ${OBLODAI_LIVE_URL:?set OBLODAI_LIVE_URL}"
  cargo test --locked --all-features -- --ignored --test-threads=1
fi

echo "all gates green"
