# Use cargo from PATH, falling back to the rustup default location.
CARGO ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)

.PHONY: format test build install

format:
	$(CARGO) fmt --all

test:
	$(CARGO) test --workspace

build:
	$(CARGO) build --workspace --release

install:
	$(CARGO) install --path crates/octotask-cli --locked
