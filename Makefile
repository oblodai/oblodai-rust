# Gates of the Rust SDK. The backend checkout (drift check, conformance suite) is $OBLODAI_BACKEND,
# else ../oblodai-backend next to this repository.

.PHONY: ci live drift fmt lint test doc

ci:            ## every gate CI runs
	./scripts/ci.sh

live:          ## the live tier too (needs OBLODAI_LIVE_URL)
	./scripts/ci.sh --live

drift:         ## fail when src/generated is stale
	./scripts/check_generated.sh --require

fmt:
	cargo fmt --all

lint:
	cargo clippy -j 2 --all-targets --all-features -- -D warnings

test:
	cargo test -j 2 --all-features

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc -j 2 --no-deps --all-features
