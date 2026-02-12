#!/bin/bash
set -euo pipefail

# Release script for gesso crates

VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name == "gesso") | .version')

echo "=== Gesso Release Script ==="
echo "Version: $VERSION"
echo ""

# Check we're on main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
    echo "Error: Must be on main branch (currently on $BRANCH)"
    exit 1
fi

# Check for uncommitted changes
if ! git diff --quiet || ! git diff --staged --quiet; then
    echo "Error: Uncommitted changes present"
    exit 1
fi

echo "Running checks..."

# Run tests
echo "  - Running tests..."
cargo test --workspace --quiet

# Run clippy
echo "  - Running clippy..."
cargo clippy --workspace --quiet -- -D warnings 2>/dev/null || true

# Dry run publish
echo "  - Checking gesso_core publish..."
cargo publish --dry-run -p gesso_core --quiet

echo "  - Checking gesso publish..."
cargo publish --dry-run -p gesso --quiet

echo ""
echo "All checks passed!"
echo ""
echo "This will publish:"
echo "  - gesso_core $VERSION"
echo "  - gesso $VERSION"
echo ""
read -p "Proceed with publish? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

echo ""
echo "Publishing gesso_core..."
cargo publish -p gesso_core

echo "Waiting for crates.io to index gesso_core..."
sleep 30

echo "Publishing gesso..."
cargo publish -p gesso

echo ""
echo "=== Release complete! ==="
echo "Published gesso_core $VERSION and gesso $VERSION"
