.PHONY: lint test coverage ci check clean

default: ci

check:
	cargo check

lint:
	cargo clippy -- -D warnings

test:
	cargo test

coverage:
	cargo llvm-cov --all-features 2>/dev/null || \
		(echo "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov" && exit 1)

ci: check lint test

clean:
	cargo clean
