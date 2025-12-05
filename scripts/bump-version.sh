#!/bin/bash
# Bump version script for Sigma Eclipse LLM
# Updates version in package.json, tauri.conf.json, and Cargo.toml

set -e

BUMP_TYPE=$1

if [[ -z "$BUMP_TYPE" ]]; then
    echo "Usage: $0 <patch|minor|major>"
    exit 1
fi

if [[ ! "$BUMP_TYPE" =~ ^(patch|minor|major)$ ]]; then
    echo "Error: Invalid bump type '$BUMP_TYPE'. Use: patch, minor, or major"
    exit 1
fi

# Get current version from package.json
CURRENT_VERSION=$(grep -o '"version": "[^"]*"' package.json | head -1 | cut -d'"' -f4)

if [[ -z "$CURRENT_VERSION" ]]; then
    echo "Error: Could not read current version from package.json"
    exit 1
fi

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Increment based on bump type
case $BUMP_TYPE in
    patch)
        PATCH=$((PATCH + 1))
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"

echo "📦 Bumping version: $CURRENT_VERSION → $NEW_VERSION"

# Update package.json
sed -i.bak "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" package.json && rm -f package.json.bak
echo "  ✅ package.json"

# Update tauri.conf.json
sed -i.bak "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json && rm -f src-tauri/tauri.conf.json.bak
echo "  ✅ src-tauri/tauri.conf.json"

# Update Cargo.toml
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" src-tauri/Cargo.toml && rm -f src-tauri/Cargo.toml.bak
echo "  ✅ src-tauri/Cargo.toml"

echo ""
echo "🎉 Version bumped to $NEW_VERSION"



