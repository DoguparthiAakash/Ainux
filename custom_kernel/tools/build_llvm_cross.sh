#!/bin/bash
set -e

# build_llvm_cross.sh
# This script downloads and builds LLVM and Clang as a cross-compiler.
# It serves as the foundation for the Ainux self-hosted compiler.

LLVM_REPO="https://github.com/llvm/llvm-project.git"
LLVM_BRANCH="release/17.x"
SRC_DIR="llvm-project"
BUILD_DIR="llvm-build"
INSTALL_DIR="$(pwd)/llvm-cross"

echo "Checking dependencies..."
for req in git cmake ninja python3; do
    if ! command -v $req &> /dev/null; then
        echo "Error: $req is required but not installed."
        exit 1
    fi
done

if [ ! -d "$SRC_DIR" ]; then
    echo "Cloning LLVM Project ($LLVM_BRANCH)..."
    # Clone with depth 1 to save massive amount of time and disk space
    git clone --depth 1 -b $LLVM_BRANCH $LLVM_REPO $SRC_DIR
else
    echo "LLVM source already exists."
fi

echo "Configuring LLVM Cross Compiler..."
mkdir -p $BUILD_DIR
cd $BUILD_DIR

# We configure LLVM to build Clang and LLD (linker)
# We target X86 primarily for Ainux, minimizing the build scope.
cmake -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DLLVM_ENABLE_PROJECTS="clang;lld" \
    -DLLVM_TARGETS_TO_BUILD="X86" \
    -DCMAKE_INSTALL_PREFIX=$INSTALL_DIR \
    -DLLVM_DEFAULT_TARGET_TRIPLE="x86_64-elf" \
    ../$SRC_DIR/llvm

echo "Configuration complete. To build, run:"
echo "cd $BUILD_DIR && ninja && ninja install"
echo ""
echo "Note: Building LLVM takes significant time and resources."
