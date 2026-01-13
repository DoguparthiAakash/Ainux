#!/bin/bash
set -e

# Build Kernel
# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic

# Build Kernel
RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-Tlinker.ld -C link-arg=src/asm/utils.o -C link-arg=src/c/hardware.o" cargo build --release --target x86_64-unknown-none

# Create ISO Structure
mkdir -p iso_root/boot
cp target/x86_64-unknown-none/release/ainux_kernel iso_root/boot/
cp limine.conf iso_root/boot/
cp limine/limine-bios.sys iso_root/boot/
cp limine/limine-bios-cd.bin iso_root/boot/
cp limine/limine-bios-cd.bin iso_root/boot/
cp limine/limine-uefi-cd.bin iso_root/boot/
cp nux_portable/standard_vision.nux iso_root/

# Create ISO
xorriso -as mkisofs -b boot/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        -o ainux.iso iso_root

echo "ISO Created: ainux.iso"
