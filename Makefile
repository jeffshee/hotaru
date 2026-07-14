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

install: $(BUILDDIR) ## Install into PREFIX (default ~/.local, no sudo); includes the scene backend if built
	meson install -C $(BUILDDIR)
	@if [ -f $(WPE_LIB) ]; then \
		install -D -m755 $(WPE_LIB) $(WPE_INSTALL_DIR)/$(notdir $(WPE_LIB)); \
		echo "Installed scene backend: $(WPE_INSTALL_DIR)/$(notdir $(WPE_LIB))"; \
	else \
		echo "Scene backend not built; run 'make wpe-lib install' to include it"; \
	fi

uninstall: $(BUILDDIR) ## Remove a previous install from PREFIX
	ninja -C $(BUILDDIR) uninstall
	rm -f $(WPE_INSTALL_DIR)/$(notdir $(WPE_LIB))

# --- Wallpaper Engine scene backend -----------------------------------------

WPE_DIR   := third_party/linux-wallpaperengine
WPE_BUILD := $(WPE_DIR)/build
WPE_LIB   := $(abspath $(WPE_BUILD)/output/liblinux-wallpaperengine-lib.so)
# hotaru's dlopen looks in <prefix>/lib{,64}/hotaru next to its own binary.
WPE_INSTALL_DIR := $(PREFIX)/lib/hotaru

# Heavy C++ translation units (glslang, quickjs) each peak at 1-2 GB, so full
# parallelism can exhaust RAM and trigger the OOM killer. Cap jobs to ~1 per
# 2 GB of currently-available memory, bounded by core count. Override with
# WPE_JOBS=N.
WPE_JOBS ?= $(shell j=$$(nproc); m=$$(awk '/MemAvailable/ {print int($$2/2000000)}' /proc/meminfo 2>/dev/null); [ -z "$$m" ] && m=$$j; [ "$$m" -lt 1 ] && m=1; [ "$$m" -lt "$$j" ] && echo $$m || echo $$j)

wpe-lib: ## Build the pinned linux-wallpaperengine scene backend (CEF-free; WPE_JOBS=N caps parallelism)
	git submodule update --init --recursive $(WPE_DIR)
	cmake -S $(WPE_DIR) -B $(WPE_BUILD) -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF -DENABLE_WEB=OFF
	cmake --build $(WPE_BUILD) --target linux-wallpaperengine-lib -j $(WPE_JOBS)
	@echo "Built $(WPE_LIB) (parallelism: $(WPE_JOBS) jobs)"
	@echo "Run 'make install' to install it, or run hotaru with: HOTARU_WPE_LIBRARY=$(WPE_LIB)"

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

.PHONY: run test lint format doc build install uninstall wpe-lib flatpak flatpak-run flatpak-uninstall clean help
