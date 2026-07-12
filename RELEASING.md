# Releasing

This package (`oblodai`) is published to **crates.io** by CI when a `v*` tag is pushed.

## Setup (one-time)
**Repo secret:** `CARGO_REGISTRY_TOKEN` — a crates.io API token.

## Cut a release
1. Bump `version` in `Cargo.toml`.
2. `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The **Release** workflow runs `cargo publish`.

CI (build + tests) runs on every push and pull request via `.github/workflows/ci.yml`.
