#!/bin/bash
set -e

# Set up paths
WORKSPACE_DIR="/mnt/e/lh/lsr/Ainux/custom_kernel"
MUSL_SRC="$WORKSPACE_DIR/external/musl"
SYSROOT="$WORKSPACE_DIR/tools/sysroot"

# Create sysroot directory if it doesn't exist
mkdir -p "$SYSROOT"

echo "Configuring musl for x86_64-linux-musl..."
cd "$MUSL_SRC"

# We configure musl to build for x86_64 using standard gcc (since our kernel provides Linux syscall ABI)
# We disable shared libraries because we only support static linking for now in Ainux.
CC=gcc AR=ar RANLIB=ranlib ./configure \
    --prefix="$SYSROOT" \
    --target=x86_64 \
    --enable-static \
    --disable-shared \
    --disable-gcc-wrapper

echo "Building musl..."
make -j$(nproc)

echo "Installing musl to sysroot..."
make install

echo "Musl build and installation complete!"
