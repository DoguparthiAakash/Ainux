#!/bin/bash
set -ex

# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
nasm -f elf64 src/asm/boot.asm -o src/asm/boot.o
nasm -f bin src/asm/ap_trampoline.asm -o src/asm/ap_trampoline.bin
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic

# Build Kernel
echo "Building Kernel..."
CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-Tlinker.ld -C link-arg=src/asm/boot.o -C link-arg=src/asm/utils.o -C link-arg=src/c/hardware.o" cargo build --release --target x86_64-unknown-none || { echo "Cargo build failed"; exit 1; }

# Build ISO
echo "Building ISO..."
rm -rf iso_root
mkdir -p iso_root/boot/grub
cp -v target/x86_64-unknown-none/release/ainux_kernel iso_root/boot/
cp -v grub.cfg iso_root/boot/grub/ || :

grub-mkrescue -o ainux.iso iso_root || { echo "ISO creation failed"; exit 1; }

# Create Disk Image (EXT4) if not exists
if [ ! -f disk2.img ]; then
    echo "Creating disk2.img (32MB)..."
    dd if=/dev/zero of=disk2.img bs=1M count=32
    mkfs.ext4 -O ^extents,^64bit -F disk2.img || { echo "mkfs.ext4 failed"; exit 1; }
fi

# Populate with hello.txt
echo "Hello World from Ext4!" > hello.txt
debugfs -w -R "write hello.txt hello.txt" disk2.img || echo "debugfs failed (optional)"

# Build and Inject Userspace Hello
echo "Building hello.c..."
gcc -static -nostdlib -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables -Ttext=0x400000 -e _start src/c/hello.c -o hello.elf
debugfs -w -R "write hello.elf hello.elf" disk2.img || echo "debugfs (hello.elf) failed"

# Detect KVM
if [ -e /dev/kvm ]; then
    ACCEL="-enable-kvm"
    echo "KVM Acceleration Enabled 🚀"
else
    ACCEL=""
    echo "KVM Not Found (Using Software Emulation - Slower)"
fi

# Run QEMU with SMP (4 cores), 2GB RAM, serial output
echo "Starting QEMU (SMP=4)..."
echo "Use Ctrl+A, X to exit."
qemu-system-x86_64 \
    -M pc \
    -smp 4 \
    -m 2G \
    -cdrom ainux.iso \
    -boot d \
    -serial stdio \
    -drive file=disk2.img,format=raw,index=0,media=disk \
    $ACCEL \
    -net nic,model=rtl8139 \
    -net user
