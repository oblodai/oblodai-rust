# Releasing

This package (`oblodai`) is published to **crates.io** by CI when a `v*` tag is pushed.

## Setup (one-time)
**Repo secret:** `CARGO_REGISTRY_TOKEN` — a crates.io API token.

## Cut a release
1. Bump `version` in `Cargo.toml`.
2. `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The **Release** workflow runs `cargo publish`.

## Before tagging

```sh
cargo fmt --all --check
python3 scripts/codegen.py --check     # src/contract matches contract/contract.json
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --all-features -- --ignored --test-threads=1
```

Refreshing the contract snapshot: replace `contract/` from the gateway's export, run
`python3 scripts/codegen.py`, and commit the regenerated `src/contract/` alongside it — the drift
gate fails otherwise.

CI (format, drift, lints, tests, examples, docs, MSRV) runs on every push and pull request via
`.github/workflows/ci.yml`.
