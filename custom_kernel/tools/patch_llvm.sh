#!/bin/bash
# Patch LLVM and Clang to recognize Ainux as an OS target

set -e

LLVM_DIR="$(cd "$(dirname "$0")/../external/llvm-project" && pwd)"

if [ ! -d "$LLVM_DIR" ]; then
    echo "LLVM submodule not found in external/llvm-project. Please clone it first."
    exit 1
fi

echo "Patching LLVM Triple.h..."
# Add Ainux to OSType enum if not present
TRIPLE_H="$LLVM_DIR/llvm/include/llvm/TargetParser/Triple.h"
if ! grep -q "Ainux" "$TRIPLE_H"; then
    sed -i '/enum OSType {/a \    Ainux,' "$TRIPLE_H"
fi

echo "Patching LLVM Triple.cpp..."
# Add Triple parsing logic
TRIPLE_CPP="$LLVM_DIR/llvm/lib/TargetParser/Triple.cpp"
if ! grep -q "ainux" "$TRIPLE_CPP"; then
    sed -i '/\.StartsWith("linux", Linux)/a \      .StartsWith("ainux", Ainux)' "$TRIPLE_CPP"
    sed -i '/case Linux:/a \  case Ainux: return "ainux";' "$TRIPLE_CPP"
fi

echo "Patching Clang ToolChains..."
# We would create clang/lib/Driver/ToolChains/Ainux.h and Ainux.cpp here.
# For now, we stub out a basic ToolChain for Ainux.
TOOLCHAIN_DIR="$LLVM_DIR/clang/lib/Driver/ToolChains"
if [ ! -f "$TOOLCHAIN_DIR/Ainux.h" ]; then
cat <<EOF > "$TOOLCHAIN_DIR/Ainux.h"
#ifndef LLVM_CLANG_LIB_DRIVER_TOOLCHAINS_AINUX_H
#define LLVM_CLANG_LIB_DRIVER_TOOLCHAINS_AINUX_H

#include "Gnu.h"
#include "clang/Driver/ToolChain.h"

namespace clang {
namespace driver {
namespace toolchains {

class LLVM_LIBRARY_VISIBILITY Ainux : public Generic_ELF {
public:
  Ainux(const Driver &D, const llvm::Triple &Triple,
          const llvm::opt::ArgList &Args);

  bool HasNativeLLVMSupport() const override { return true; }
};

} // end namespace toolchains
} // end namespace driver
} // end namespace clang

#endif
EOF
fi

if [ ! -f "$TOOLCHAIN_DIR/Ainux.cpp" ]; then
cat <<EOF > "$TOOLCHAIN_DIR/Ainux.cpp"
#include "Ainux.h"
#include "CommonArgs.h"

using namespace clang::driver;
using namespace clang::driver::toolchains;

Ainux::Ainux(const Driver &D, const llvm::Triple &Triple,
                 const llvm::opt::ArgList &Args)
    : Generic_ELF(D, Triple, Args) {}
EOF
fi

echo "Ainux Patches applied to LLVM successfully."
