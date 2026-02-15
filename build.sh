#!/bin/bash

# Production build script for Linkr
# This script builds an optimized release binary

set -e

echo "Building Linkr for production..."
echo ""

# Check if in correct directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Run this from the project root."
    exit 1
fi

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Run tests (optional, uncomment if you add tests)
# echo "Running tests..."
# cargo test --release

# Build release binary
echo "Building release binary..."
cargo build --release

# Check binary size
BINARY_PATH="target/release/linkr"
if [ -f "$BINARY_PATH" ]; then
    BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    echo ""
    echo "Build successful!"
    echo "Binary size: $BINARY_SIZE"
    echo "Location: $BINARY_PATH"
    echo ""
    echo "To run locally:"
    echo "  export DATABASE_URL='postgres://user:pass@localhost/linkr'"
    echo "  export PUBLIC_BASE_URL='http://localhost:3000'"
    echo "  ./target/release/linkr"
else
    echo "Build failed - binary not found"
    exit 1
fi
