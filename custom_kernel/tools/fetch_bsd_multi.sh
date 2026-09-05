#!/bin/bash
set -e

# fetch_bsd_multi.sh
# This script downloads specific subsystems from the major BSDs to be integrated
# into the Ainux Multi-Kernel architecture.

TARGET_DIR="$(pwd)/src/bsd_external"
mkdir -p "$TARGET_DIR"
cd "$TARGET_DIR"

echo "Fetching FreeBSD sources (Focus: Stability & VFS)..."
# Download a tarball of a stable FreeBSD release (e.g., 14.0)
if [ ! -d "freebsd" ]; then
    mkdir freebsd
    echo "Cloning shallow FreeBSD tree..."
    git clone --depth 1 https://github.com/freebsd/freebsd-src.git freebsd
else
    echo "FreeBSD source already present."
fi

echo "Fetching OpenBSD sources (Focus: Security & Crypto)..."
if [ ! -d "openbsd" ]; then
    mkdir openbsd
    echo "Cloning shallow OpenBSD tree..."
    git clone --depth 1 https://github.com/openbsd/src.git openbsd
else
    echo "OpenBSD source already present."
fi

echo "Fetching NetBSD sources (Focus: Networking)..."
if [ ! -d "netbsd" ]; then
    mkdir netbsd
    echo "Cloning shallow NetBSD tree..."
    git clone --depth 1 https://github.com/NetBSD/src.git netbsd
else
    echo "NetBSD source already present."
fi

echo "==========================================="
echo "BSD Multi-Kernel subsystems fetched successfully."
echo "The source trees are located in src/bsd_external/"
echo "You can now begin wiring the C source files into build.rs via src/bsd_compat/"
