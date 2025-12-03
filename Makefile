# Sigma Eclipse LLM - Build Makefile
# Build commands

.PHONY: all dev build build-signed build-host build-all build-windows build-windows-signed clean install-deps help version bump-patch bump-minor bump-major publish check-signing-key

# Detect OS and architecture
UNAME_S := $(shell uname -s)

ifeq ($(UNAME_S),Darwin)
    TARGET := aarch64-apple-darwin
    BINARY_EXT :=
else ifeq ($(UNAME_S),Linux)
    TARGET := x86_64-unknown-linux-gnu
    BINARY_EXT :=
else
    TARGET := x86_64-pc-windows-msvc
    BINARY_EXT := .exe
endif

# Directories
TAURI_DIR := src-tauri
BINARIES_DIR := $(TAURI_DIR)/binaries
HOST_BINARY := sigma-eclipse-host
HOST_BINARY_PATH := $(BINARIES_DIR)/$(HOST_BINARY)-$(TARGET)$(BINARY_EXT)

# Updater signing key path
SIGNING_KEY_PATH := ~/.tauri/sigma-eclipse-llm.key

# Default target
all: build

# Install dependencies
install-deps:
	@echo "📦 Installing dependencies..."
	npm install

# Development mode
dev: ensure-host-binary
	@echo "🚀 Starting development server..."
	npm run tauri dev

# Build native messaging host binary
build-host: ensure-binaries-dir
	@echo "🔧 Building native messaging host for $(TARGET)..."
	cd $(TAURI_DIR) && cargo build --release --bin $(HOST_BINARY)
	cp $(TAURI_DIR)/target/release/$(HOST_BINARY)$(BINARY_EXT) $(HOST_BINARY_PATH)
	chmod +x $(HOST_BINARY_PATH)
	@echo "✅ Host binary built: $(HOST_BINARY_PATH)"

# Ensure binaries directory exists with placeholders
ensure-binaries-dir:
	@mkdir -p $(BINARIES_DIR)
	@touch $(BINARIES_DIR)/$(HOST_BINARY)-aarch64-apple-darwin
	@touch $(BINARIES_DIR)/$(HOST_BINARY)-x86_64-unknown-linux-gnu
	@touch $(BINARIES_DIR)/$(HOST_BINARY)-x86_64-pc-windows-msvc.exe

# Ensure host binary exists (build if not)
ensure-host-binary: ensure-binaries-dir
	@if [ ! -s $(HOST_BINARY_PATH) ]; then \
		echo "🔧 Host binary not found, building..."; \
		$(MAKE) build-host; \
	fi

# Build production release (without signing)
build: ensure-host-binary
	@echo "🏗️  Building production release..."
	npm run tauri build
	@echo "✅ Build complete!"
	@echo "📦 Bundles available at: $(TAURI_DIR)/target/release/bundle/"

# Build production release with updater signing
# Set TAURI_SIGNING_PRIVATE_KEY_PASSWORD env var if your key has a password
build-signed: ensure-host-binary check-signing-key
	@echo "🏗️  Building signed production release..."
	@echo "🔐 Using signing key: $(SIGNING_KEY_PATH)"
	@if [ -n "$$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" ]; then \
		echo "🔑 Using password from TAURI_SIGNING_PRIVATE_KEY_PASSWORD env"; \
	else \
		echo "🔓 No password set (TAURI_SIGNING_PRIVATE_KEY_PASSWORD is empty)"; \
	fi
	TAURI_SIGNING_PRIVATE_KEY="$$(cat $$(eval echo $(SIGNING_KEY_PATH)))" \
	npm run tauri build
	@echo "✅ Signed build complete!"
	@echo "📦 Bundles available at: $(TAURI_DIR)/target/release/bundle/"

# Check if signing key exists
check-signing-key:
	@if [ ! -f "$$(eval echo $(SIGNING_KEY_PATH))" ]; then \
		echo "❌ Error: Signing key not found at $(SIGNING_KEY_PATH)"; \
		echo "   Generate one with: npx tauri signer generate -w $(SIGNING_KEY_PATH)"; \
		exit 1; \
	fi
	@echo "✅ Signing key found"

# Build everything (host + app)
build-all: build-host build

# Build for specific platform (macOS ARM64)
build-macos: ensure-host-binary
	@echo "🍎 Building for macOS (ARM64)..."
	npm run tauri build

# Build for Windows using Docker (unsigned)
build-windows:
	@echo "🪟 Building for Windows using Docker..."
	@if ! docker info > /dev/null 2>&1; then \
		echo "❌ Error: Docker is not running. Please start Docker Desktop."; \
		exit 1; \
	fi
	docker-compose build build-windows
	docker-compose run --rm build-windows
	@echo "✅ Windows build complete!"
	@echo "📦 Bundles available at: $(TAURI_DIR)/target/x86_64-pc-windows-gnu/release/bundle/"

# Build for Windows using Docker with updater signing
build-windows-signed: check-signing-key
	@echo "🪟 Building signed Windows release using Docker..."
	@if ! docker info > /dev/null 2>&1; then \
		echo "❌ Error: Docker is not running. Please start Docker Desktop."; \
		exit 1; \
	fi
	@echo "🔐 Using signing key: $(SIGNING_KEY_PATH)"
	docker-compose build build-windows
	TAURI_SIGNING_PRIVATE_KEY="$$(cat $$(eval echo $(SIGNING_KEY_PATH)))" \
	TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" \
	docker-compose run --rm \
		-e TAURI_SIGNING_PRIVATE_KEY \
		-e TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
		build-windows
	@echo "✅ Signed Windows build complete!"
	@echo "📦 Bundles available at: $(TAURI_DIR)/target/x86_64-pc-windows-gnu/release/bundle/"

# Build host for all platforms (requires cross-compilation setup)
build-host-all: ensure-binaries-dir
	@echo "🔧 Building host for all platforms..."
	@echo "⚠️  Note: Cross-compilation requires additional setup"
	cd $(TAURI_DIR) && cargo build --release --bin $(HOST_BINARY)
	cp $(TAURI_DIR)/target/release/$(HOST_BINARY)$(BINARY_EXT) $(HOST_BINARY_PATH)
	chmod +x $(HOST_BINARY_PATH)

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cd $(TAURI_DIR) && cargo clean
	rm -rf dist
	rm -rf $(BINARIES_DIR)/*.exe $(BINARIES_DIR)/*-darwin $(BINARIES_DIR)/*-linux-gnu
	@echo "✅ Clean complete!"

# Clean and rebuild
rebuild: clean build-all

# Run tests
test:
	@echo "🧪 Running tests..."
	cd $(TAURI_DIR) && cargo test

# Check code without building
check: ensure-binaries-dir
	@echo "🔍 Checking code..."
	cd $(TAURI_DIR) && cargo check
	npm run lint 2>/dev/null || true

# Format code
fmt:
	@echo "✨ Formatting code..."
	cd $(TAURI_DIR) && cargo fmt
	npm run format 2>/dev/null || true

# Show current version
version:
	@grep -o '"version": "[^"]*"' package.json | head -1 | cut -d'"' -f4

# Bump patch version (0.1.0 -> 0.1.1)
bump-patch:
	@./scripts/bump-version.sh patch

# Bump minor version (0.1.0 -> 0.2.0)
bump-minor:
	@./scripts/bump-version.sh minor

# Bump major version (0.1.0 -> 1.0.0)
bump-major:
	@./scripts/bump-version.sh major

# Create git tag with current version and push
publish:
	@VERSION=$$(grep -o '"version": "[^"]*"' package.json | head -1 | cut -d'"' -f4); \
	echo "🏷️  Creating tag v$$VERSION..."; \
	git tag -a "v$$VERSION" -m "Release v$$VERSION"; \
	echo "📤 Pushing tag v$$VERSION..."; \
	git push origin "v$$VERSION"; \
	echo "✅ Published v$$VERSION"

# Show help
help:
	@echo "Sigma Eclipse LLM - Build Commands"
	@echo "=================================="
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  all           - Build production release (default)"
	@echo "  dev           - Start development server"
	@echo "  build         - Build production release (unsigned)"
	@echo "  build-signed  - Build production release with updater signing"
	@echo "  build-host    - Build native messaging host binary"
	@echo "  build-all     - Build host and app"
	@echo "  build-macos   - Build macOS ARM64 binary"
	@echo "  build-windows - Build Windows binary using Docker (unsigned)"
	@echo "  build-windows-signed - Build signed Windows binary using Docker"
	@echo "  install-deps  - Install npm dependencies"
	@echo "  clean         - Clean build artifacts"
	@echo "  rebuild       - Clean and rebuild everything"
	@echo "  test          - Run tests"
	@echo "  check         - Check code without building"
	@echo "  fmt           - Format code"
	@echo "  version       - Show current version"
	@echo "  bump-patch    - Bump patch version (0.1.0 -> 0.1.1)"
	@echo "  bump-minor    - Bump minor version (0.1.0 -> 0.2.0)"
	@echo "  bump-major    - Bump major version (0.1.0 -> 1.0.0)"
	@echo "  publish       - Create git tag with current version and push"
	@echo "  help          - Show this help message"
	@echo ""
	@echo "Current target: $(TARGET)"
	@echo "Host binary: $(HOST_BINARY_PATH)"
	@echo "Signing key: $(SIGNING_KEY_PATH)"

