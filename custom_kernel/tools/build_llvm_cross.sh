#!/bin/bash
# Build LLVM Cross-Compiler for MithlOS

set -e

LLVM_DIR="$(cd "$(dirname "$0")/../external/llvm-project" && pwd)"
BUILD_DIR="$LLVM_DIR/build_cross"

if [ ! -d "$LLVM_DIR" ]; then
    echo "LLVM submodule not found in external/llvm-project."
    exit 1
fi

mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

echo "Configuring LLVM for Cross-Compilation targeting MithlOS..."
cmake -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DLLVM_TARGETS_TO_BUILD="X86" \
    -DLLVM_ENABLE_PROJECTS="clang;lld" \
    -DCMAKE_INSTALL_PREFIX="/usr/local/mithlos-toolchain" \
    ../llvm

echo "Configuration complete. To build, run: cd $BUILD_DIR && make -j$(nproc)"
