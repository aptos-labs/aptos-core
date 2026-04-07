#!/bin/sh
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

########################################################
# Download and install a binary release from GitHub    #
########################################################
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- <binary-name> [options]
#
# Examples:
#   # Install latest version
#   curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- aptos-node
#
#   # Install specific version
#   curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- aptos-node --version 1.2.3
#
#   # Install to custom directory
#   curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- aptos-node --bin-dir /usr/local/bin

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
BINARY_NAME=""
VERSION="latest"
BIN_DIR="$HOME/.local/bin"
FORCE=false
YES=false
REPO="aptos-labs/aptos-core"
GITHUB_API="https://api.github.com"
GITHUB_RELEASES="https://github.com"

# Functions
print_error() {
    printf "%bError: %s%b\n" "$RED" "$1" "$NC" >&2
}

print_success() {
    printf "%b%s%b\n" "$GREEN" "$1" "$NC"
}

print_info() {
    printf "%b%s%b\n" "$BLUE" "$1" "$NC"
}

print_warning() {
    printf "%b%s%b\n" "$YELLOW" "$1" "$NC"
}

usage() {
    cat << EOF
Usage: $0 <binary-name> [options]

Options:
    --version <version>     Install specific version (default: latest)
    --bin-dir <path>        Installation directory (default: ~/.local/bin)
    -f, --force            Force reinstall even if already installed
    -y, --yes              Skip confirmation prompts
    -h, --help             Show this help message

Examples:
    $0 aptos-node
    $0 aptos-node --version 1.2.3
    $0 aptos-debugger --bin-dir /usr/local/bin

EOF
    exit 1
}

# Parse arguments
BINARY_NAME="$1"
shift || usage

while [ $# -gt 0 ]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --bin-dir)
            BIN_DIR="$2"
            shift 2
            ;;
        -f|--force)
            FORCE=true
            shift
            ;;
        -y|--yes)
            YES=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            ;;
    esac
done

# Validate binary name
if [ -z "$BINARY_NAME" ]; then
    print_error "Binary name is required"
    usage
fi

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map to Rust target triple
case "$OS" in
    linux)
        case "$ARCH" in
            x86_64)
                TARGET_TRIPLE="x86_64-unknown-linux-gnu"
                ;;
            aarch64|arm64)
                TARGET_TRIPLE="aarch64-unknown-linux-gnu"
                ;;
            *)
                print_error "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    darwin)
        case "$ARCH" in
            x86_64)
                TARGET_TRIPLE="x86_64-apple-darwin"
                ;;
            arm64)
                TARGET_TRIPLE="aarch64-apple-darwin"
                ;;
            *)
                print_error "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        print_error "Unsupported operating system: $OS"
        exit 1
        ;;
esac

print_info "Installing $BINARY_NAME for $TARGET_TRIPLE..."

# Get version if latest
if [ "$VERSION" = "latest" ]; then
    print_info "Fetching latest release version..."
    # Fetch with pagination support (100 per page, max 3 pages = 300 releases)
    VERSION=""
    for page in 1 2 3; do
        RELEASES=$(curl -fsSL "$GITHUB_API/repos/$REPO/releases?per_page=100&page=$page")
        VERSION=$(echo "$RELEASES" | \
                  grep -o "\"tag_name\": \"$BINARY_NAME-v[0-9.]*\"" | \
                  head -n 1 | \
                  sed 's/.*v\([0-9.]*\).*/\1/')

        if [ -n "$VERSION" ]; then
            break
        fi

        # Check if there are more pages
        if ! echo "$RELEASES" | grep -q "\"tag_name\""; then
            break
        fi
    done

    if [ -z "$VERSION" ]; then
        print_error "Could not determine latest version for $BINARY_NAME"
        exit 1
    fi
    print_info "Latest version: $VERSION"
fi

# Check if already installed
INSTALLED_PATH="$BIN_DIR/$BINARY_NAME"
if [ -f "$INSTALLED_PATH" ] && [ "$FORCE" != "true" ]; then
    CURRENT_VERSION=$("$INSTALLED_PATH" --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -n 1 || echo "unknown")
    if [ "$CURRENT_VERSION" = "$VERSION" ]; then
        print_success "$BINARY_NAME $VERSION is already installed"
        exit 0
    else
        print_warning "$BINARY_NAME is already installed (version: $CURRENT_VERSION)"
        if [ "$YES" != "true" ]; then
            printf "Do you want to upgrade to version %s? [y/N] " "$VERSION"
            read -r REPLY
            case "$REPLY" in
                [Yy]|[Yy][Ee][Ss])
                    ;;
                *)
                    exit 0
                    ;;
            esac
        fi
    fi
fi

# Construct download URL
RELEASE_TAG="${BINARY_NAME}-v${VERSION}"
ARCHIVE_NAME="${BINARY_NAME}-v${VERSION}-${TARGET_TRIPLE}.zip"
DOWNLOAD_URL="$GITHUB_RELEASES/$REPO/releases/download/$RELEASE_TAG/$ARCHIVE_NAME"
CHECKSUM_URL="$GITHUB_RELEASES/$REPO/releases/download/$RELEASE_TAG/$ARCHIVE_NAME.sha256"

print_info "Downloading from: $DOWNLOAD_URL"

# Create temporary directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT INT TERM

# Download archive
cd "$TMP_DIR"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_NAME"; then
    print_error "Failed to download $ARCHIVE_NAME"
    print_error "URL: $DOWNLOAD_URL"
    print_info "Available releases: $GITHUB_RELEASES/$REPO/releases"
    exit 1
fi

# Download and verify checksum if available
if curl -fsSL "$CHECKSUM_URL" -o "$ARCHIVE_NAME.sha256" 2>/dev/null; then
    print_info "Verifying checksum..."
    if command -v shasum >/dev/null 2>&1; then
        if ! shasum -a 256 -c "$ARCHIVE_NAME.sha256" 2>/dev/null; then
            print_error "Checksum verification failed"
            exit 1
        fi
    elif command -v sha256sum >/dev/null 2>&1; then
        if ! sha256sum -c "$ARCHIVE_NAME.sha256" 2>/dev/null; then
            print_error "Checksum verification failed"
            exit 1
        fi
    else
        print_warning "Neither shasum nor sha256sum found, skipping checksum verification"
    fi
    print_success "Checksum verified"
else
    print_warning "Checksum not available, skipping verification"
fi

# Extract archive
print_info "Extracting archive..."
unzip -q "$ARCHIVE_NAME"

# Locate extracted binary
EXTRACTED_BINARY_PATH=""
if [ -f "$BINARY_NAME" ]; then
    EXTRACTED_BINARY_PATH="$BINARY_NAME"
else
    # Some archives place binaries in a subdirectory; search for the expected name
    EXTRACTED_BINARY_PATH=$(find . -maxdepth 3 -type f -name "$BINARY_NAME" 2>/dev/null | head -n 1)
fi

if [ -z "$EXTRACTED_BINARY_PATH" ]; then
    print_error "Extracted binary '$BINARY_NAME' not found in archive. Please verify the release contents."
    exit 1
fi

# Create bin directory if it doesn't exist
mkdir -p "$BIN_DIR"

# Install binary
print_info "Installing to $BIN_DIR/$BINARY_NAME..."
cp "$EXTRACTED_BINARY_PATH" "$BIN_DIR/$BINARY_NAME"
chmod +x "$BIN_DIR/$BINARY_NAME"

# Verify installation
if [ -f "$BIN_DIR/$BINARY_NAME" ]; then
    print_success "Successfully installed $BINARY_NAME $VERSION"

    # Check if bin directory is in PATH
    case ":$PATH:" in
        *":$BIN_DIR:"*)
            print_info "Run '$BINARY_NAME --version' to verify the installation"
            ;;
        *)
            print_warning "$BIN_DIR is not in your PATH"
            print_info "Add it to your PATH by running:"
            print_info "  echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.profile"
            print_info "  . ~/.profile"
            ;;
    esac
else
    print_error "Installation failed"
    exit 1
fi
