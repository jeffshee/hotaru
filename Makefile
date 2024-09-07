BUILDDIR := builddir

# Default target
.PHONY: all
all: build

# Setup build directory (system)
.PHONY: setup
setup:
	meson setup $(BUILDDIR)

# Setup build directory (local)
.PHONY: setup-local
setup-local:
	meson setup $(BUILDDIR) --prefix=$(HOME)/.local

# Build the project
.PHONY: build
build:
	meson compile -C $(BUILDDIR)

# Install the application
.PHONY: install
install: build
	meson install -C $(BUILDDIR)

# Run the application
.PHONY: run
run: install
	cargo run

# Uninstall the application
.PHONY: uninstall
uninstall:
	meson uninstall -C $(BUILDDIR)

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

# Help target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  setup        - Set up the build directory (system)"
	@echo "  setup-local  - Set up the build directory (local)"
	@echo "  build        - Build the project"
	@echo "  install      - Install the application"
	@echo "  run          - Run the application"
	@echo "  uninstall    - Uninstall the application"
	@echo "  test         - Run tests"
	@echo "  doc          - Generate and open documentation"
	@echo "  clean        - Clean build artifacts"
