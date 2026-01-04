#!/bin/bash
set -ex

# Build Kernel
# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic

# Build Kernel
echo "Building Kernel..."
RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-Tlinker.ld -C link-arg=src/asm/utils.o -C link-arg=src/c/hardware.o" cargo build --release --target x86_64-unknown-none || { echo "Cargo build failed"; exit 1; }

# Build ISO
echo "Building ISO..."
mkdir -p iso_root
cp -v target/x86_64-unknown-none/release/ainux_kernel iso_root/
cp -v limine.cfg limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/

xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o ainux.iso || { echo "ISO creation failed"; exit 1; }

# Create Disk Image (EXT4) if not exists
if [ ! -f disk.img ]; then
    echo "Creating disk.img (32MB)..."
    dd if=/dev/zero of=disk.img bs=1M count=32
    mkfs.ext4 -F disk.img || { echo "mkfs.ext4 failed"; exit 1; }
fi

# Run QEMU with serial output to console
echo "Starting QEMU..."
echo "Use Ctrl+A, X to exit."
qemu-system-x86_64 -M q35 -m 2G -cdrom ainux.iso -boot d -serial stdio -drive file=disk.img,format=raw,index=0,media=disk
