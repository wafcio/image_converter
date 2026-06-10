.PHONY: deps lint test coverage ci check clean

UNAME := $(shell uname -s)

default: ci

deps:
	@if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists dav1d 2>/dev/null; then \
		echo "✓ system deps found"; \
		exit 0; \
	fi
	@if [ "$(UNAME)" = "Darwin" ] && command -v brew >/dev/null 2>&1; then \
		echo "Installing system dependencies via Homebrew..."; \
		brew install dav1d pkg-config; \
	elif [ "$(UNAME)" = "Linux" ] && command -v apt-get >/dev/null 2>&1; then \
		echo "Run: sudo apt-get install -y libdav1d-dev pkg-config"; \
		exit 1; \
	elif [ "$(UNAME)" = "Linux" ] && command -v dnf >/dev/null 2>&1; then \
		echo "Run: sudo dnf install -y libdav1d-devel pkg-config"; \
		exit 1; \
	else \
		echo "Please install dav1d and pkg-config for your platform"; \
		exit 1; \
	fi

check:
	cargo check

lint:
	cargo clippy -- -D warnings

test:
	cargo test

coverage:
	cargo llvm-cov --all-features 2>/dev/null || \
		(echo "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov" && exit 1)

ci: deps check lint test

clean:
	cargo clean
