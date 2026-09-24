#!/bin/bash
set -e

KERNEL_DIR="$(cd "$(dirname "$0")/.." && pwd)"
MUSL_DIR="$KERNEL_DIR/musl-libc"
LLVM_INSTALL_DIR="$KERNEL_DIR/compilers/llvm_ainux"
SYSROOT_DIR="$MUSL_DIR/sysroot"

export PATH="$LLVM_INSTALL_DIR/bin:$PATH"

if ! command -v clang &> /dev/null; then
    echo "Clang not found! Make sure you run build_llvm_ainux.sh first to build the compiler."
    exit 1
fi

echo "Configuring musl-libc for Ainux..."

cd "$MUSL_DIR"

# Clean previous build if any
make clean || true

# We specify x86_64-ainux as the target.
# Since we are using Clang, we pass CC="clang" and use the LLVM tools.
./configure \
    --prefix="$SYSROOT_DIR" \
    --target=x86_64-unknown-linux-musl \
    CC="clang -target x86_64-unknown-linux-musl -fuse-ld=lld" \
    AR="llvm-ar" \
    RANLIB="llvm-ranlib"

echo "Building musl-libc..."
make -j$(nproc)

echo "Installing musl-libc to sysroot..."
make install

echo "Musl-libc built and installed successfully to $SYSROOT_DIR!"
