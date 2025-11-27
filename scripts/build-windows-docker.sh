#!/bin/bash

# Script to build Windows version using Docker
# Usage: ./scripts/build-windows-docker.sh

set -e

echo "🔧 Building Sigma Eclipse for Windows using Docker..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker is not running. Please start Docker Desktop.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Docker is running${NC}"

# Build Docker image
echo ""
echo "📦 Building Docker image..."
docker-compose build build-windows

# Run the build
echo ""
echo "🔨 Starting Windows build..."
docker-compose run --rm build-windows

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Build completed successfully!${NC}"
    echo ""
    echo "📁 Output files are in:"
    echo "   src-tauri/target/x86_64-pc-windows-gnu/release/bundle/"
    echo ""
    echo "You can find:"
    echo "   - .msi installer"
    echo "   - .exe executable"
    echo ""
else
    echo ""
    echo -e "${RED}✗ Build failed${NC}"
    echo ""
    echo "To debug, run:"
    echo "   docker-compose run --rm shell"
    exit 1
fi

