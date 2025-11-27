#!/bin/bash
# Build script for Native Messaging Host binary
# This script builds the host binary and copies it to the binaries directory
# with the correct target triple suffix required by Tauri's externalBin

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TAURI_DIR="$PROJECT_ROOT/src-tauri"
BINARIES_DIR="$TAURI_DIR/binaries"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🔧 Building Native Messaging Host${NC}"
echo "=================================="

# Detect target triple (macOS ARM64 only)
if [[ "$OSTYPE" == "darwin"* ]]; then
    TARGET="aarch64-apple-darwin"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    TARGET="x86_64-unknown-linux-gnu"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    TARGET="x86_64-pc-windows-msvc"
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

echo "Target: $TARGET"

# Create binaries directory if it doesn't exist
mkdir -p "$BINARIES_DIR"

# Create placeholder files for Tauri build system (it checks externalBin paths)
# These will be replaced with actual binaries after build
echo -e "${YELLOW}Creating placeholder files for Tauri...${NC}"
touch "$BINARIES_DIR/sigma-eclipse-host-aarch64-apple-darwin"
touch "$BINARIES_DIR/sigma-eclipse-host-x86_64-unknown-linux-gnu"
touch "$BINARIES_DIR/sigma-eclipse-host-x86_64-pc-windows-msvc.exe"

# Build the binary
echo -e "${YELLOW}Building sigma-eclipse-host...${NC}"
cd "$TAURI_DIR"
cargo build --release --bin sigma-eclipse-host

# Copy with target triple suffix
SOURCE="$TAURI_DIR/target/release/sigma-eclipse-host"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    SOURCE="${SOURCE}.exe"
    DEST="$BINARIES_DIR/sigma-eclipse-host-${TARGET}.exe"
else
    DEST="$BINARIES_DIR/sigma-eclipse-host-${TARGET}"
fi

echo "Copying $SOURCE -> $DEST"
cp "$SOURCE" "$DEST"

# Make executable
chmod +x "$DEST"

echo -e "${GREEN}✓ Native Messaging Host built successfully${NC}"
echo "Binary location: $DEST"

