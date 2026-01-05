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
rm -rf iso_root
mkdir -p iso_root
cp -v target/x86_64-unknown-none/release/ainux_kernel iso_root/
cp -v limine.cfg limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/

xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o ainux.iso || { echo "ISO creation failed"; exit 1; }

# Create Disk Image (EXT4) if not exists
# Create Disk Image (EXT4) if not exists
if [ ! -f disk.img ]; then
    echo "Creating disk.img (32MB)..."
    dd if=/dev/zero of=disk.img bs=1M count=32
    mkfs.ext4 -O ^extents,^64bit -F disk.img || { echo "mkfs.ext4 failed"; exit 1; }
fi

# Populate with hello.txt
echo "Hello World from Ext4!" > hello.txt
# Use debugfs to write file (requires e2fsprogs)
debugfs -w -R "write hello.txt hello.txt" disk.img || echo "debugfs failed (optional)"

# Build and Inject Userspace Hello
echo "Building hello.c..."
gcc -static -nostdlib -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables -Ttext=0x400000 -e _start src/c/hello.c -o hello.elf
debugfs -w -R "write hello.elf hello.elf" disk.img || echo "debugfs (hello.elf) failed"

# Inject init.anux (Full System Init)
./tools/nuxc.py init.nuxi hello.anux
debugfs -w -R "write hello.anux hello.anux" disk.img || echo "debugfs (hello.anux) failed"

# Stat root to check blocks (Commented out to prevent pager blocking)
# debugfs -R "stat /" disk.img
# debugfs -R "stat /hello.txt" disk.img

# Run QEMU with serial output to console
echo "Ensuring fresh build..."
./build_iso.sh
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo "Starting QEMU..."
echo "Use Ctrl+A, X to exit."
qemu-system-x86_64 -M pc -m 2G -cdrom ainux.iso -boot d -serial stdio -drive file=disk.img,format=raw,index=0,media=disk
