# Use cargo from PATH, falling back to the rustup default location.
CARGO ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)

# Same resolution as the binary: an empty OCTOTASK_HOME counts as unset.
DATA_DIR := $(or $(OCTOTASK_HOME),$(HOME)/.octotask)

.PHONY: format test build install clean-data install-clean

format:
	$(CARGO) fmt --all

test:
	$(CARGO) test --workspace

build:
	$(CARGO) build --workspace --release

install:
	$(CARGO) install --path crates/octotask-cli --locked

clean-data:
	@printf 'Delete %s? [y/N] ' "$(DATA_DIR)"; \
	read answer; \
	if [ "$$answer" = y ]; then \
		rm -rf "$(DATA_DIR)" && echo "deleted $(DATA_DIR)"; \
	else \
		echo aborted; \
	fi

# Install first, so a failed build leaves the data untouched.
install-clean: install clean-data
