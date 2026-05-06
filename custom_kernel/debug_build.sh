#!/bin/bash
export PATH=$PATH:/home/mute/.cargo/bin
cargo build --release --target x86_64-unknown-none 2>&1
