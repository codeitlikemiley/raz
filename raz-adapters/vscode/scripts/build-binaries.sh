#!/bin/bash
# Build RAZ CLI binaries for different platforms

set -e

echo "Building RAZ CLI binaries for VS Code extension..."

# Get the project root (parent of raz-adapters directory)
PROJECT_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
VSCODE_DIR="$PROJECT_ROOT/raz-adapters/vscode"
BIN_DIR="$VSCODE_DIR/bin"

# Clean and create bin directory
rm -rf "$BIN_DIR"
mkdir -p "$BIN_DIR"

# Build for current platform (for development)
echo "Building for current platform..."
cd "$PROJECT_ROOT"
cargo build -p raz-cli --release

# Detect current platform and copy binary
if [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="darwin"
    if [[ $(uname -m) == "arm64" ]]; then
        ARCH="arm64"
    else
        ARCH="x64"
    fi
    mkdir -p "$BIN_DIR/$PLATFORM-$ARCH"
    cp "$PROJECT_ROOT/target/release/raz" "$BIN_DIR/$PLATFORM-$ARCH/raz"
    chmod +x "$BIN_DIR/$PLATFORM-$ARCH/raz"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="linux"
    ARCH="x64"
    mkdir -p "$BIN_DIR/$PLATFORM-$ARCH"
    cp "$PROJECT_ROOT/target/release/raz" "$BIN_DIR/$PLATFORM-$ARCH/raz"
    chmod +x "$BIN_DIR/$PLATFORM-$ARCH/raz"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    PLATFORM="win32"
    ARCH="x64"
    mkdir -p "$BIN_DIR/$PLATFORM-$ARCH"
    cp "$PROJECT_ROOT/target/release/raz.exe" "$BIN_DIR/$PLATFORM-$ARCH/raz.exe"
fi

echo "Binary built for $PLATFORM-$ARCH"

# Cross-compile for all major platforms
echo "Building for all supported platforms..."

# Linux x64
echo "Building for Linux x64..."
if cargo build -p raz-cli --release --target x86_64-unknown-linux-gnu; then
    mkdir -p "$BIN_DIR/linux-x64"
    cp "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/raz" "$BIN_DIR/linux-x64/raz"
    chmod +x "$BIN_DIR/linux-x64/raz"
    echo "✓ Linux x64 build successful"
else
    echo "✗ Linux x64 build failed"
fi

# macOS x64  
echo "Building for macOS x64..."
if cargo build -p raz-cli --release --target x86_64-apple-darwin; then
    mkdir -p "$BIN_DIR/darwin-x64"
    cp "$PROJECT_ROOT/target/x86_64-apple-darwin/release/raz" "$BIN_DIR/darwin-x64/raz"
    chmod +x "$BIN_DIR/darwin-x64/raz"
    echo "✓ macOS x64 build successful"
else
    echo "✗ macOS x64 build failed"
fi

# macOS ARM64 (current platform build should handle this)
if [[ "$PLATFORM-$ARCH" != "darwin-arm64" ]]; then
    echo "Building for macOS ARM64..."
    if cargo build -p raz-cli --release --target aarch64-apple-darwin; then
        mkdir -p "$BIN_DIR/darwin-arm64"
        cp "$PROJECT_ROOT/target/aarch64-apple-darwin/release/raz" "$BIN_DIR/darwin-arm64/raz"
        chmod +x "$BIN_DIR/darwin-arm64/raz"
        echo "✓ macOS ARM64 build successful"
    else
        echo "✗ macOS ARM64 build failed"
    fi
fi

# Windows x64
echo "Building for Windows x64..."
if cargo build -p raz-cli --release --target x86_64-pc-windows-msvc; then
    mkdir -p "$BIN_DIR/win32-x64"
    cp "$PROJECT_ROOT/target/x86_64-pc-windows-msvc/release/raz.exe" "$BIN_DIR/win32-x64/raz.exe"
    echo "✓ Windows x64 build successful"
else
    echo "✗ Windows x64 build failed"
fi

echo "Build complete! Binaries available in: $BIN_DIR"
ls -la "$BIN_DIR"