#!/bin/bash
# Cross-platform build script for Nux compiler

set -e

echo "Building Nux for multiple platforms..."

cd nux_portable

# Build for Linux (x86_64)
echo "Building for Linux x86_64..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/nux ../builds/nux-linux-x86_64

# Build for Windows (x86_64)
echo "Building for Windows x86_64..."
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/nux.exe ../builds/nux-windows-x86_64.exe

# Build for macOS (x86_64 Intel)
echo "Building for macOS x86_64 (Intel)..."
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/nux ../builds/nux-macos-x86_64

# Build for macOS (ARM64 Apple Silicon)
echo "Building for macOS ARM64 (Apple Silicon)..."
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/nux ../builds/nux-macos-arm64

echo "All builds complete! Binaries are in ../builds/"
ls -lh ../builds/
