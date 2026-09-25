#!/bin/bash
export PATH="$PATH:$HOME/.cargo/bin"
LINK_ARGS="-C code-model=kernel -C relocation-model=static -C link-arg=-z -C link-arg=max-page-size=0x1000 -C link-arg=-Tlinker.ld"
CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="$LINK_ARGS" cargo build --release --offline --target x86_64-unknown-none
