# Dev convenience wrapper. Run `make` or `make help` for the target list.
# Override BUILDDIR=... to change the Meson build directory.

BUILDDIR    ?= _build
PREFIX      ?= $(HOME)/.local
MESON_FLAGS ?=
FLATPAK_DIR ?= build-flatpak
MANIFEST    := pkgs/flatpak/io.github.jeffshee.Hotaru.json

.DEFAULT_GOAL := help

# --- Cargo dev environment --------------------------------------------------

run: ## Run in debug mode (cargo run)
	cargo run

test: ## Run the test suite
	cargo test

lint: ## Lint the sources with clippy
	cargo clippy

format: ## Auto-format the sources with rustfmt
	cargo fmt

doc: ## Generate and open the API documentation
	cargo doc --no-deps --open

# --- Meson build ------------------------------------------------------------

$(BUILDDIR):
	meson setup --prefix=$(PREFIX) $(MESON_FLAGS) $(BUILDDIR)

build: $(BUILDDIR) ## Configure (if needed) and compile (MESON_FLAGS="-Dmpv=false" to skip libmpv)
	meson compile -C $(BUILDDIR)

install: $(BUILDDIR) ## Install into PREFIX (default ~/.local, no sudo)
	meson install -C $(BUILDDIR)

uninstall: $(BUILDDIR) ## Remove a previous install from PREFIX
	ninja -C $(BUILDDIR) uninstall

# --- Flatpak ----------------------------------------------------------------

flatpak: ## Build & install the Flatpak (pulls SDK/Platform from flathub)
	flatpak-builder --user --install --force-clean \
		--install-deps-from=flathub $(FLATPAK_DIR) $(MANIFEST)

flatpak-run: ## Run the installed Flatpak
	flatpak run io.github.jeffshee.Hotaru

flatpak-uninstall: ## Uninstall the Flatpak
	flatpak --user uninstall io.github.jeffshee.Hotaru

# --- Housekeeping -----------------------------------------------------------

clean: ## Remove build directories and tool caches
	rm -rf $(BUILDDIR) $(FLATPAK_DIR) .flatpak .flatpak-builder
	cargo clean

help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.PHONY: run test lint format doc build install uninstall flatpak flatpak-run flatpak-uninstall clean help
