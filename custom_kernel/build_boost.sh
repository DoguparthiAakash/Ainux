#!/bin/bash
set -e

WORKSPACE_DIR="/mnt/e/lh/lsr/Ainux/custom_kernel"
SYSROOT="$WORKSPACE_DIR/tools/sysroot"
BOOST_VERSION="1.84.0"
BOOST_VERSION_UNDERSCORE="1_84_0"

echo "Downloading Boost $BOOST_VERSION..."
cd $WORKSPACE_DIR/external
if [ ! -d "boost_${BOOST_VERSION_UNDERSCORE}" ]; then
    wget -O boost.tar.gz https://archives.boost.io/release/${BOOST_VERSION}/source/boost_${BOOST_VERSION_UNDERSCORE}.tar.gz

    tar -xf boost.tar.gz
    rm -f boost.tar.gz
fi

cd boost_${BOOST_VERSION_UNDERSCORE}

echo "Bootstrapping Boost..."
./bootstrap.sh --prefix=$SYSROOT --with-toolset=clang

# Write the user-config.jam for cross-compilation
cat <<EOF > user-config.jam
using clang : musl
    : clang++
    : <compileflags>"--target=x86_64-linux-musl --sysroot=$SYSROOT -nostdinc++ -isystem $SYSROOT/include/c++/v1 -isystem $SYSROOT/include"
      <linkflags>"--target=x86_64-linux-musl --sysroot=$SYSROOT -nostdlib++ -L$SYSROOT/lib -lc++ -lc++abi -lunwind"

    ;
EOF

echo "Building and installing Boost..."
./b2 --user-config=user-config.jam \
     toolset=clang-musl \
     target-os=linux \
     architecture=x86 \
     address-model=64 \
     link=static \
     threading=multi \
     variant=release \
     install

echo "Boost build complete!"
