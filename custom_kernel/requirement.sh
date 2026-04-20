#!/bin/bash

# requirement.sh - Setup script for Ainux Kernel Development
# This script installs necessary tools for building, emulating, and developing the kernel.
# Targeted for Ubuntu/Debian based systems (including WSL).

set -e

echo "--- Updating Package Lists ---"
sudo apt-get update

echo "--- Installing Build Essentials & Compilers ---"
# build-essential: make, gcc, etc.
# nasm: Assembly compiler
# gcc-multilib: Support for different architectures/headers
# zig: Needed for TUI modules (installed later if missing)
sudo apt-get install -y build-essential nasm gcc-multilib git curl wget

echo "--- Installing ISO & Disk Utilities ---"
# xorriso, mtools, grub-pc-bin: Required by grub-mkrescue
# e2fsprogs: Required for mkfs.ext4 and debugfs
sudo apt-get install -y xorriso mtools grub-common grub-pc-bin grub-efi-amd64-bin e2fsprogs

echo "--- Installing Emulator ---"
sudo apt-get install -y qemu-system-x86

echo "--- Installing Hybrid Compiler Dependencies (Python/LLVM) ---"
sudo apt-get install -y python3 python3-pip clang llvm

echo "--- Checking Rust Installation ---"
if ! command -v rustup &> /dev/null; then
    echo "Rustup not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Rustup already installed. Updating..."
    rustup update
fi

echo "--- Configuring Rust for Kernel Development ---"
rustup target add x86_64-unknown-none
rustup component add rust-src

echo "--- Installing Zig Compiler ---"
# Zig version 0.11.0+ is recommended for the TUI module.
if ! command -v zig &> /dev/null; then
    echo "Zig not found. Attempting to install via snap..."
    if command -v snap &> /dev/null; then
        sudo snap install zig --classic --beta || echo "Snap installation failed."
    else
        echo "Snap not found. Please install Zig manually (version 0.11.0+) from https://ziglang.org/download/"
    fi
else
    echo "Zig already installed: $(zig version)"
fi

echo "--- Setup Complete! ---"
echo "IMPORTANT: Please run 'source \$HOME/.cargo/env' or restart your terminal to update your PATH."
