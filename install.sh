#!/usr/bin/env bash
set -e

REPO="B-bsw/springboot-initalizr-CLI"
BIN_NAME="spring-init"

echo "Installing $BIN_NAME..."

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Darwin" ]; then
    OS="apple-darwin"
elif [ "$OS" = "Linux" ]; then
    OS="unknown-linux-gnu"
else
    echo "Unsupported OS: $OS"
    exit 1
fi

if [ "$ARCH" = "x86_64" ]; then
    ARCH="x86_64"
elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
    ARCH="aarch64"
else
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

TARGET="${ARCH}-${OS}"

# Fetch latest release data
LATEST_RELEASE_URL="https://api.github.com/repos/$REPO/releases/latest"
echo "Fetching latest release information..."
LATEST_VERSION=$(curl -s "$LATEST_RELEASE_URL" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo "Error: Could not determine latest release version. Make sure you have created a release on GitHub."
    exit 1
fi

echo "Latest version is $LATEST_VERSION"

# Download the binary archive
FILE_NAME="${BIN_NAME}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/${LATEST_VERSION}/${FILE_NAME}"

TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

echo "Downloading $DOWNLOAD_URL..."
if curl -fsSL -o "$FILE_NAME" "$DOWNLOAD_URL"; then
    echo "Download complete."
else
    echo "Error: Failed to download $DOWNLOAD_URL"
    exit 1
fi

echo "Extracting..."
tar -xzf "$FILE_NAME"

# Install binary
INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

echo "Installing to $INSTALL_DIR/$BIN_NAME..."
mv "$BIN_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BIN_NAME"

# Clean up
rm -rf "$TMP_DIR"

echo "====================================="
echo "✅ $BIN_NAME installed successfully!"
echo "You can now run it by typing: $BIN_NAME"

if [[ "$INSTALL_DIR" == "$HOME/.local/bin" ]]; then
    if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
        echo "⚠️  Note: $HOME/.local/bin is not in your PATH."
        echo "Please add it to your ~/.bashrc or ~/.zshrc:"
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
fi
