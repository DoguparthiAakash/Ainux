#!/bin/bash
set -e

LLVM_DIR="$(cd "$(dirname "$0")/../external/llvm-project" && pwd)"
BUILD_DIR="$LLVM_DIR/build_ainux_host"
INSTALL_DIR="$(cd "$(dirname "$0")/.." && pwd)/compilers/llvm_ainux"

echo "Building LLVM (Clang) for Host with Ainux Target Support..."

mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

cmake -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DLLVM_ENABLE_PROJECTS="clang;lld" \
    -DLLVM_TARGETS_TO_BUILD="X86" \
    -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR" \
    -DLLVM_INCLUDE_TESTS=OFF \
    -DLLVM_INCLUDE_EXAMPLES=OFF \
    ../llvm

# Building with -j$(nproc) might consume too much RAM on some machines
echo "Starting compilation (this will take a while)..."
make -j4
make install

echo "LLVM Build completed. Installed to $INSTALL_DIR"
