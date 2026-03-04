#!/bin/bash
set -euo pipefail

# Release script for motif crates
#
# Usage:
#   ./scripts/release.sh                   - Publish current version (prompts for confirmation)
#   ./scripts/release.sh bump <version>    - Bump version in Cargo.toml, commit, and tag
#
# The CI release workflow (.github/workflows/release.yml) automates validation
# and crates.io publishing when you push a tag. Use this script locally to:
#   1. Bump the workspace version and create a tag (bump subcommand)
#   2. Run a local publish dry-run (default, no args)

SUBCOMMAND="${1:-publish}"

# ── Bump subcommand ──────────────────────────────────────────────────────────
if [ "$SUBCOMMAND" = "bump" ]; then
  NEW_VERSION="${2:-}"
  if [ -z "$NEW_VERSION" ]; then
    echo "Usage: $0 bump <version>" >&2
    echo "Example: $0 bump 0.1.0" >&2
    exit 1
  fi

  if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: version must be in semver format (e.g. 0.1.0)" >&2
    exit 1
  fi

  # Must be on main with a clean tree
  BRANCH=$(git branch --show-current)
  if [ "$BRANCH" != "main" ]; then
    echo "Error: must be on main branch (currently on $BRANCH)" >&2
    exit 1
  fi
  if ! git diff --quiet || ! git diff --staged --quiet; then
    echo "Error: uncommitted changes present" >&2
    exit 1
  fi

  echo "Bumping version to $NEW_VERSION..."

  # Update [workspace.package] version in root Cargo.toml
  # Uses BSD/GNU compatible in-place sed (writes .bak then removes it)
  sed -i.bak "s/^version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" Cargo.toml
  rm -f Cargo.toml.bak

  # Commit and tag
  git add Cargo.toml
  git commit -m "chore: bump version to v$NEW_VERSION"
  git tag "v$NEW_VERSION"

  echo ""
  echo "Version bumped to v$NEW_VERSION and tagged."
  echo "Push the commit and tag to trigger the release workflow:"
  echo "  git push && git push --tags"
  exit 0
fi

# ── Publish subcommand (default) ─────────────────────────────────────────────
VERSION=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name == "motif") | .version')

echo "=== Motif Release Script ==="
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
echo "  - Checking motif_core publish..."
cargo publish --dry-run -p motif_core --quiet

echo "  - Checking motif publish..."
cargo publish --dry-run -p motif --quiet

echo ""
echo "All checks passed!"
echo ""
echo "This will publish:"
echo "  - motif_core $VERSION"
echo "  - motif $VERSION"
echo ""
read -p "Proceed with publish? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

echo ""
echo "Publishing motif_core..."
cargo publish -p motif_core

echo "Waiting for crates.io to index motif_core..."
sleep 30

echo "Publishing motif..."
cargo publish -p motif

echo ""
echo "=== Release complete! ==="
echo "Published motif_core $VERSION and motif $VERSION"
