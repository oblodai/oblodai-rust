# Releasing

This package (`oblodai`) is published to **crates.io** when a `v*` tag is pushed.

## Setup (one-time)
**Repo secret:** `CARGO_REGISTRY_TOKEN` — a crates.io API token.

## Cut a release
1. Regenerate from the gateway's contract (`make sdk` in the backend) and commit `src/generated/`
   and `names.lock` together.
2. Bump `version` in `Cargo.toml`, add the CHANGELOG entry.
3. `OBLODAI_BACKEND=../oblodai-backend make ci` — every gate green.
4. `git tag vX.Y.Z && git push origin vX.Y.Z`.

## Before tagging

```sh
OBLODAI_BACKEND=../oblodai-backend make ci                      # drift, fmt, clippy, tests, docs, package, MSRV
OBLODAI_LIVE_URL=http://127.0.0.1:8095 make live                # the live tier against a real gateway
```

`make ci` fails when `src/generated/` differs from what the generator makes of the backend's
`services/core/api/openapi.json` — fix it by regenerating, never by hand.
