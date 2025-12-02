# Sigma Eclipse LLM - Build Makefile
# Священные команды сборки во славу Омниссии

.PHONY: all dev build build-host build-all build-windows clean install-deps help version bump-patch bump-minor bump-major

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

# Build production release
build: ensure-host-binary
	@echo "🏗️  Building production release..."
	npm run tauri build
	@echo "✅ Build complete!"
	@echo "📦 Bundles available at: $(TAURI_DIR)/target/release/bundle/"

# Build everything (host + app)
build-all: build-host build

# Build for specific platform (macOS ARM64)
build-macos: ensure-host-binary
	@echo "🍎 Building for macOS (ARM64)..."
	npm run tauri build

# Build for Windows using Docker
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
	@echo "  build         - Build production release"
	@echo "  build-host    - Build native messaging host binary"
	@echo "  build-all     - Build host and app"
	@echo "  build-macos   - Build macOS ARM64 binary"
	@echo "  build-windows - Build Windows binary using Docker"
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
	@echo "  help          - Show this help message"
	@echo ""
	@echo "Current target: $(TARGET)"
	@echo "Host binary: $(HOST_BINARY_PATH)"

