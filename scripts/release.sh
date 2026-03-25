#!/bin/sh
set -eu

# Release script for cq
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.2.0

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

VERSION="$1"
TAG="v${VERSION}"

# Validate version format
if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: version must be in semver format (e.g. 0.2.0)"
    exit 1
fi

# Ensure clean working tree
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: working tree is not clean. Commit or stash changes first."
    exit 1
fi

# Ensure we're on main or master
BRANCH="$(git branch --show-current)"
if [ "$BRANCH" != "main" ] && [ "$BRANCH" != "master" ]; then
    echo "Warning: releasing from branch '$BRANCH' (not main/master)"
    printf "Continue? [y/N] "
    read -r answer
    if [ "$answer" != "y" ] && [ "$answer" != "Y" ]; then
        exit 1
    fi
fi

# Check tag doesn't already exist
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Error: tag $TAG already exists"
    exit 1
fi

# Update version in Cargo.toml
echo "Updating Cargo.toml version to ${VERSION}..."
sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

# Update Cargo.lock
cargo check --quiet 2>/dev/null

# Verify it builds
echo "Running checks..."
cargo fmt --check
cargo clippy -- -D warnings
cargo test

# Commit version bump
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to ${VERSION}"

# Create and push tag
echo "Creating tag ${TAG}..."
git tag -a "$TAG" -m "Release ${TAG}"

echo ""
echo "Ready to release! Run:"
echo "  git push && git push --tags"
echo ""
echo "This will trigger the GitHub Actions release workflow."
