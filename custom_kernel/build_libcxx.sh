#!/bin/bash
set -e

WORKSPACE_DIR="/mnt/e/lh/lsr/Ainux/custom_kernel"
LLVM_SRC="$WORKSPACE_DIR/external/llvm-project"
SYSROOT="$WORKSPACE_DIR/tools/sysroot"

echo "Building libc++ and libc++abi for Ainux using Musl sysroot..."

cd "$LLVM_SRC"
mkdir -p build-libcxx
cd build-libcxx

# Configure CMake for runtimes (libcxx and libcxxabi)
# We set CMAKE_SYSTEM_NAME to Linux so it uses standard POSIX paths, which Ainux mimics.
cmake -G "Unix Makefiles" \
    -DCMAKE_SYSTEM_NAME=Linux \
    -DCMAKE_C_COMPILER_WORKS=ON \
    -DCMAKE_CXX_COMPILER_WORKS=ON \
    -DCMAKE_C_COMPILER=clang \
    -DCMAKE_CXX_COMPILER=clang++ \
    -DCMAKE_C_FLAGS="--target=x86_64-linux-musl --sysroot=$SYSROOT" \
    -DCMAKE_CXX_FLAGS="--target=x86_64-linux-musl --sysroot=$SYSROOT" \
    -DCMAKE_SYSROOT="$SYSROOT" \
    -DCMAKE_INSTALL_PREFIX="$SYSROOT" \
    -DLLVM_ENABLE_RUNTIMES="libcxx;libcxxabi;libunwind" \
    -DLIBCXX_ENABLE_SHARED=OFF \
    -DLIBCXX_ENABLE_STATIC=ON \
    -DLIBCXXABI_ENABLE_SHARED=OFF \
    -DLIBCXXABI_ENABLE_STATIC=ON \
    -DLIBUNWIND_ENABLE_SHARED=OFF \
    -DLIBUNWIND_ENABLE_STATIC=ON \
    -DLIBCXX_INCLUDE_BENCHMARKS=OFF \
    -DLIBCXX_INCLUDE_TESTS=OFF \
    -DLIBCXXABI_ENABLE_THREADS=ON \
    -DLIBCXX_ENABLE_THREADS=ON \
    ../runtimes

echo "Compiling libunwind, libc++ and libc++abi..."
make -j$(nproc) unwind cxx cxxabi

echo "Installing to sysroot..."
make install-unwind install-cxx install-cxxabi

echo "libc++ build complete!"
