# Use cargo from PATH, falling back to the rustup default location.
CARGO ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)

# Same resolution as the binary: an empty OCTOTASK_HOME counts as unset.
DATA_DIR := $(or $(OCTOTASK_HOME),$(HOME)/.octotask)

# The octok of this checkout: an installed one may be older than the project files.
OCTOK := $(CARGO) run --quiet --locked -p octotask --

.PHONY: format test build install docs docs-serve docs-reference clean-data install-clean

format:
	$(CARGO) fmt --all

test:
	$(CARGO) test --workspace

build:
	$(CARGO) build --workspace --release

install:
	$(CARGO) install --path crates/octotask-cli --locked

# Needs mdbook on PATH: .github/scripts/install-mdbook.sh installs the version CI uses.
docs:
	.github/scripts/build-docs.sh

docs-serve:
	mdbook serve docs

# The committed docs/src/reference/commands.md is this output; a test fails when it is stale.
docs-reference:
	$(OCTOK) docs reference/commands > target/commands.md
	mv target/commands.md docs/src/reference/commands.md

# The projects go first: only octok knows which folders they have on other machines.
# If one cannot be deleted, this stops before the data folder goes, so its project file is kept.
clean-data:
	@printf 'Delete %s, and the folders of its projects on other machines? [y/N] ' "$(DATA_DIR)"; \
	read answer; \
	if [ "$$answer" != y ]; then echo aborted; exit 0; fi; \
	for file in "$(DATA_DIR)"/projects/*.yaml; do \
		[ -e "$$file" ] || continue; \
		project=$$(basename "$$file" .yaml); \
		OCTOTASK_HOME="$(DATA_DIR)" $(OCTOK) project rm --force "$$project" >/dev/null || { \
			echo "project $$project was not deleted: $(DATA_DIR) is kept"; \
			exit 1; \
		}; \
		echo "deleted project $$project on every machine"; \
	done; \
	rm -rf "$(DATA_DIR)" && echo "deleted $(DATA_DIR)"

# Install first, so a failed build leaves the data untouched.
install-clean: install clean-data
