#!/bin/bash
set -e

# Release script for repo_to_text
# Tags a release - GitHub Actions handles building and uploading artifacts

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: ./release.sh <version>"
    echo "Example: ./release.sh 0.2.0"
    exit 1
fi

# Ensure we're on main and up to date
echo "==> Checking git status..."
git fetch origin
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Working directory is not clean. Commit or stash your changes."
    exit 1
fi

# Update version in Cargo.toml
echo "==> Updating version to $VERSION in Cargo.toml..."
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# Verify it builds
echo "==> Verifying build..."
cargo build --release

# Commit version bump
echo "==> Committing version bump..."
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $VERSION"

# Create tag
echo "==> Creating git tag v$VERSION..."
git tag -a "v$VERSION" -m "Release v$VERSION"

echo ""
echo "==> Release preparation complete!"
echo ""
echo "Next steps:"
echo "  git push origin main --tags"
echo ""
echo "GitHub Actions will automatically:"
echo "  - Build binaries for macOS (x64/ARM), Linux (x64/ARM), and Windows"
echo "  - Create a GitHub release with all artifacts"
echo "  - Generate checksums"
