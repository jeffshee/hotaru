BUILDDIR := builddir
SRC_DIR := src
LICENSE_HEADER := data/license-header.txt
AUTHOR ?= Jeff Shee

# Setup build directory
.PHONY: setup
setup:
	meson setup $(BUILDDIR)

# Setup build directory (local)
.PHONY: setup-local
setup-local:
	meson setup $(BUILDDIR) --prefix=$(HOME)/.local

# Build the application
.PHONY: build
build:
	meson compile -C $(BUILDDIR)

# Run the application
.PHONY: run
run:
	cargo run

# Install the application
.PHONY: install
install: build
	meson install -C $(BUILDDIR)

# Uninstall the application
.PHONY: uninstall
uninstall:
	ninja uninstall -C $(BUILDDIR)

# Run tests
.PHONY: test
test: build
	meson test -C $(BUILDDIR)

# Generate and open documentation
.PHONY: doc
doc:
	cargo doc --no-deps --open

# Clean build artifacts
.PHONY: clean
clean:
	rm -rf $(BUILDDIR)
	cargo clean

# Generate a new .rs file with license header
%.rs:
	@if [ ! -f $(SRC_DIR)/$@ ]; then \
		mkdir -p $(SRC_DIR); \
		echo "Generating $(SRC_DIR)/$@"; \
		year=$$(date +%Y); \
		sed -e "s/{filename}/$@/" \
			-e "s/{year}/$${year}/" \
			-e "s/{author}/$(AUTHOR)/" \
			$(LICENSE_HEADER) > $(SRC_DIR)/$@; \
		echo "" >> $(SRC_DIR)/$@; \
		echo "// Add your code here" >> $(SRC_DIR)/$@; \
		echo "File $(SRC_DIR)/$@ created with license header."; \
	else \
		echo "File $(SRC_DIR)/$@ already exists. Skipping."; \
	fi

# Flatpak targets
.PHONY: install-flathub-repo
install-flathub-repo:
	flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo

.PHONY: flatpak-clean
flatpak-clean:
	rm -rf .flatpak .flatpak-builder

.PHONY: flatpak-build
flatpak-build: install-flathub-repo
	flatpak-builder --force-clean --user --install-deps-from=flathub \
		--ccache --disable-updates --repo=.flatpak/repo \
		.flatpak/build pkgs/flatpak/io.github.jeffshee.Hotaru.json

.PHONY: flatpak-run
flatpak-run:
	flatpak run io.github.jeffshee.Hotaru

.PHONY: flatpak-install
flatpak-install: flatpak-build
	flatpak --user remote-add --if-not-exists --no-gpg-verify hotaru .flatpak/repo
	flatpak --user install --reinstall hotaru io.github.jeffshee.Hotaru

.PHONY: flatpak-uninstall
flatpak-uninstall:
	flatpak --user uninstall io.github.jeffshee.Hotaru

# Help target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  setup        - Set up the build directory"
	@echo "  setup-local  - Set up the build directory (local)"
	@echo "  build        - Build the application"
	@echo "  run          - Run the application"
	@echo "  install      - Build and install the application"
	@echo "  uninstall    - Uninstall the application"
	@echo "  test         - Run tests"
	@echo "  doc          - Generate and open documentation"
	@echo "  clean        - Clean build artifacts"
	@echo "  make *.rs    - Generate a new .rs file with license header"
	@echo "  flatpak-clean      - Clean Flatpak build artifacts"
	@echo "  flatpak-build      - Build Flatpak package"
	@echo "  flatpak-run        - Run Flatpak package"
	@echo "  flatpak-install    - Build and install Flatpak package"
	@echo "  flatpak-uninstall  - Uninstall Flatpak package"
