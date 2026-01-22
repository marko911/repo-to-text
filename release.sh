#!/bin/bash
set -e

# Release script for repo_to_text
# Creates GitHub releases with pre-built binaries for macOS and Linux

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

# Build release binary for current platform
echo "==> Building release binary..."
cargo build --release

# Create release directory
RELEASE_DIR="releases/v$VERSION"
mkdir -p "$RELEASE_DIR"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Normalize architecture name
case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Create tarball
BINARY_NAME="repo_to_text-v$VERSION-$OS-$ARCH"
echo "==> Creating $BINARY_NAME.tar.gz..."
cp target/release/repo_to_text "$RELEASE_DIR/$BINARY_NAME"
cd "$RELEASE_DIR"
tar -czvf "$BINARY_NAME.tar.gz" "$BINARY_NAME"
rm "$BINARY_NAME"
cd - > /dev/null

# Generate SHA256 checksum
echo "==> Generating checksums..."
cd "$RELEASE_DIR"
shasum -a 256 *.tar.gz > checksums.txt
cd - > /dev/null

# Commit version bump
echo "==> Committing version bump..."
git add Cargo.toml
git commit -m "chore: bump version to $VERSION"

# Create and push tag
echo "==> Creating git tag v$VERSION..."
git tag -a "v$VERSION" -m "Release v$VERSION"

echo ""
echo "==> Release preparation complete!"
echo ""
echo "Next steps:"
echo "  1. Push changes:  git push origin main --tags"
echo "  2. Create GitHub release at: https://github.com/marko911/repo_to_text/releases/new"
echo "  3. Upload artifacts from: $RELEASE_DIR/"
echo ""
echo "Release artifacts:"
ls -la "$RELEASE_DIR/"
