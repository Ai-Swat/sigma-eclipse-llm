#!/bin/bash

# Script to open interactive shell in Docker container for debugging
# Usage: ./scripts/docker-shell.sh

echo "🐚 Opening interactive shell in Docker container..."
echo ""
echo "Available commands inside container:"
echo "  - npm run tauri build -- --target x86_64-pc-windows-gnu"
echo "  - cargo build --target x86_64-pc-windows-gnu"
echo "  - cargo check --target x86_64-pc-windows-gnu"
echo ""
echo "Type 'exit' to leave the container."
echo ""

docker-compose run --rm shell

