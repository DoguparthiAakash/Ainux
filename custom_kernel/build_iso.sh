#!/bin/bash
set -e

# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
nasm -f elf64 src/asm/boot.asm -o src/asm/boot.o
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic

# Build Kernel
echo "Building Kernel..."
CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-Tlinker.ld -C link-arg=src/asm/boot.o -C link-arg=src/asm/utils.o -C link-arg=src/c/hardware.o" cargo build --release --target x86_64-unknown-none

# Create ISO Structure
rm -rf iso_root
mkdir -p iso_root/boot/grub
cp target/x86_64-unknown-none/release/ainux_kernel iso_root/boot/
cp grub.cfg iso_root/boot/grub/

# Create ISO
grub-mkrescue -o ainux.iso iso_root

echo "ISO Created: ainux.iso"
