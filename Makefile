.PHONY: lint test coverage ci check clean

lint:
	cargo clippy -- -D warnings

test:
	cargo test

coverage:
	cargo llvm-cov --all-features 2>/dev/null || \
		(echo "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov" && exit 1)

ci: lint test

check:
	cargo check

clean:
	cargo clean
