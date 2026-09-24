#!/bin/bash
set -e

KERNEL_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LLVM_DIR="$KERNEL_DIR/external/llvm-project"
BUILD_DIR="$LLVM_DIR/build_ainux_native"

# This is the path to the cross-compiler we built in Step 1
CROSS_COMPILER_DIR="$KERNEL_DIR/compilers/llvm_ainux"
SYSROOT_DIR="$KERNEL_DIR/musl-libc/sysroot"

# This is where the final native binaries (clang.elf, lld.elf) will be placed
OUTPUT_DIR="$KERNEL_DIR/disk3_root/usr/bin"

export PATH="$CROSS_COMPILER_DIR/bin:$PATH"

if [ ! -f "$CROSS_COMPILER_DIR/bin/clang" ]; then
    echo "Host cross-compiler not found! Run build_llvm_ainux.sh first."
    exit 1
fi

if [ ! -f "$SYSROOT_DIR/lib/libc.a" ]; then
    echo "Ainux musl-libc not found! Run build_musl_ainux.sh first."
    exit 1
fi

echo "Building Native LLVM (clang.elf) for Ainux..."

mkdir -p "$BUILD_DIR"
mkdir -p "$OUTPUT_DIR"
cd "$BUILD_DIR"

# Here we use CMake to tell LLVM to compile ITSELF using our cross-compiler.
# The resulting binaries will only run on Ainux.
cmake -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_SYSTEM_NAME=Linux \
    -DCMAKE_SYSTEM_PROCESSOR=x86_64 \
    -DCMAKE_C_COMPILER="$CROSS_COMPILER_DIR/bin/clang" \
    -DCMAKE_CXX_COMPILER="$CROSS_COMPILER_DIR/bin/clang++" \
    -DCMAKE_C_COMPILER_WORKS=1 \
    -DCMAKE_CXX_COMPILER_WORKS=1 \
    -DHAVE_CXX_ATOMICS_WITHOUT_LIB=1 \
    -DHAVE_CXX_ATOMICS64_WITHOUT_LIB=1 \
    -DCMAKE_C_FLAGS="--sysroot=$SYSROOT_DIR -target x86_64-unknown-linux-musl -fuse-ld=lld" \
    -DCMAKE_CXX_FLAGS="--sysroot=$SYSROOT_DIR -target x86_64-unknown-linux-musl -fuse-ld=lld -stdlib=libc++ -cxx-isystem $SYSROOT_DIR/include/c++/v1" \
    -DLLVM_ENABLE_PROJECTS="clang;lld" \
    -DLLVM_TARGETS_TO_BUILD="X86" \
    -DCMAKE_EXE_LINKER_FLAGS="-static -fuse-ld=lld -stdlib=libc++ -lc++abi -lunwind -lgcc" \
    -DLLVM_ENABLE_LIBCXX=ON \
    -DLLVM_INCLUDE_TESTS=OFF \
    -DLLVM_INCLUDE_EXAMPLES=OFF \
    -DLLVM_INCLUDE_BENCHMARKS=OFF \
    -DLLVM_BUILD_STATIC=ON \
    "$LLVM_DIR/llvm"

echo "Compiling native clang and lld..."
make -j$(nproc) clang lld

# Copy the generated ELF files into the disk root so they end up on the ISO/Image
echo "Copying clang and lld into Ainux filesystem..."
cp bin/clang "$OUTPUT_DIR/clang.elf"
cp bin/lld "$OUTPUT_DIR/lld.elf"

# Also copy the sysroot into the disk image so the compiler has standard headers
mkdir -p "$KERNEL_DIR/disk3_root/usr/include"
mkdir -p "$KERNEL_DIR/disk3_root/usr/lib"
cp -r "$SYSROOT_DIR/include/"* "$KERNEL_DIR/disk3_root/usr/include/"
cp -r "$SYSROOT_DIR/lib/"* "$KERNEL_DIR/disk3_root/usr/lib/"

echo "Success! clang.elf and lld.elf are now ready to be packaged into the OS image."
