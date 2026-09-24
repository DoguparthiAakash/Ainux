#!/bin/bash
set -e

KERNEL_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LLVM_DIR="$KERNEL_DIR/external/llvm-project"
BUILD_DIR="$LLVM_DIR/build_ainux_runtimes"
SYSROOT_DIR="$KERNEL_DIR/musl-libc/sysroot"
CROSS_COMPILER_DIR="$KERNEL_DIR/compilers/llvm_ainux"

export PATH="$CROSS_COMPILER_DIR/bin:$PATH"

if [ ! -f "$CROSS_COMPILER_DIR/bin/clang" ]; then
    echo "Host cross-compiler not found! Run build_llvm_ainux.sh first."
    exit 1
fi

if [ ! -f "$SYSROOT_DIR/lib/libc.a" ]; then
    echo "Ainux musl-libc not found! Run build_musl_ainux.sh first."
    exit 1
fi

echo "Copying Linux kernel headers to sysroot (required for C++ runtimes)..."
mkdir -p "$SYSROOT_DIR/include"
cp -rL -n /usr/include/linux "$SYSROOT_DIR/include/" || true
cp -rL -n /usr/include/asm "$SYSROOT_DIR/include/" || true
cp -rL -n /usr/include/asm-generic "$SYSROOT_DIR/include/" || true

echo "Building LLVM runtimes (libc++, libc++abi, libunwind) for Ainux..."

mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# Clean old build
rm -rf * || true

# Compile the runtimes natively for Ainux
cmake -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_C_COMPILER="$CROSS_COMPILER_DIR/bin/clang" \
    -DCMAKE_CXX_COMPILER="$CROSS_COMPILER_DIR/bin/clang++" \
    -DCMAKE_C_FLAGS="--sysroot=$SYSROOT_DIR -target x86_64-unknown-linux-musl -fuse-ld=lld" \
    -DCMAKE_CXX_FLAGS="--sysroot=$SYSROOT_DIR -target x86_64-unknown-linux-musl -fuse-ld=lld" \
    -DCMAKE_C_COMPILER_WORKS=1 \
    -DCMAKE_CXX_COMPILER_WORKS=1 \
    -DCMAKE_SYSTEM_NAME=Linux \
    -DLLVM_ENABLE_RUNTIMES="compiler-rt;libunwind;libcxxabi;libcxx" \
    -DLIBCXX_ENABLE_SHARED=OFF \
    -DLIBCXX_ENABLE_STATIC=ON \
    -DLIBCXXABI_ENABLE_SHARED=OFF \
    -DLIBCXXABI_ENABLE_STATIC=ON \
    -DLIBUNWIND_ENABLE_SHARED=OFF \
    -DLIBUNWIND_ENABLE_STATIC=ON \
    -DLIBCXX_HAS_MUSL_LIBC=ON \
    -DLIBCXX_INCLUDE_BENCHMARKS=OFF \
    -DCMAKE_INSTALL_PREFIX="$SYSROOT_DIR" \
    ../runtimes

echo "Compiling runtimes..."
make -j$(nproc)

echo "Installing runtimes to sysroot..."
make install

echo "Success! C++ standard library (libc++) installed to sysroot."
