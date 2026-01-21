#!/bin/bash
set -ex

# Build Kernel
# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic

# Build Kernel
echo "Building Kernel..."
CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-Tlinker.ld -C link-arg=src/asm/utils.o -C link-arg=src/c/hardware.o" cargo build --release --target x86_64-unknown-none || { echo "Cargo build failed"; exit 1; }

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
if [ ! -f disk2.img ]; then
    echo "Creating disk2.img (32MB)..."
    dd if=/dev/zero of=disk2.img bs=1M count=32
    mkfs.ext4 -O ^extents,^64bit -F disk2.img || { echo "mkfs.ext4 failed"; exit 1; }
fi

# Populate with hello.txt
echo "Hello World from Ext4!" > hello.txt
# Use debugfs to write file (requires e2fsprogs)
debugfs -w -R "write hello.txt hello.txt" disk2.img || echo "debugfs failed (optional)"

# Build and Inject Userspace Hello
echo "Building hello.c..."
gcc -static -nostdlib -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables -Ttext=0x400000 -e _start src/c/hello.c -o hello.elf
debugfs -w -R "write hello.elf hello.elf" disk2.img || echo "debugfs (hello.elf) failed"

# Inject init.anux (Full System Init)
# ./tools/nuxc.py init.nuxi hello.anux # Use kernel compiler now

echo "Injecting Nux Scripts..."
# Create library directory
debugfs -w -R "mkdir lib" disk2.img || true

# Inject Libraries
debugfs -w -R "write nux_portable/io.nux lib/io.nux" disk2.img || echo "Failed to inject io.nux"
debugfs -w -R "write nux_portable/math.nux lib/math.nux" disk2.img || echo "Failed to inject math.nux"
debugfs -w -R "write nux_portable/util.nux lib/util.nux" disk2.img || echo "Failed to inject util.nux"
debugfs -w -R "write nux_portable/graphics.nux lib/graphics.nux" disk2.img || echo "Failed to inject graphics.nux"
debugfs -w -R "write nux_portable/collections.nux lib/collections.nux" disk2.img || echo "Failed to inject collections.nux"
debugfs -w -R "write nux_portable/string.nux lib/string.nux" disk2.img || echo "Failed to inject string.nux"
# Create Library Structure
debugfs -w -R "mkdir lib/util" disk2.img || true
debugfs -w -R "mkdir lib/io" disk2.img || true
debugfs -w -R "mkdir lib/lang" disk2.img || true
debugfs -w -R "mkdir lib/data" disk2.img || true

# Inject Libraries (Categorized)
# IO
debugfs -w -R "write nux_portable/io.nux lib/io/console.nux" disk2.img || echo "Failed io.nux"
debugfs -w -R "write nux_portable/file.nux lib/io/file.nux" disk2.img || echo "Failed file.nux"

debugfs -w -R "write nux_portable/math.nux lib/util/math.nux" disk2.img || echo "Failed math.nux"
debugfs -w -R "write nux_portable/io.nux lib/io.nux" disk2.img || echo "Failed io.nux" 
debugfs -w -R "write nux_portable/file.nux lib/file.nux" disk2.img || echo "Failed file.nux"
debugfs -w -R "write nux_portable/sys.nux lib/sys.nux" disk2.img || echo "Failed sys.nux"
debugfs -w -R "write nux_portable/memory.nux lib/memory.nux" disk2.img || echo "Failed memory.nux"
debugfs -w -R "write nux_portable/string.nux lib/string.nux" disk2.img || echo "Failed string.nux"
debugfs -w -R "write nux_portable/collections.nux lib/collections.nux" disk2.img || echo "Failed collections.nux"
debugfs -w -R "write nux_portable/time.nux lib/time.nux" disk2.img || echo "Failed time.nux"
debugfs -w -R "write nux_portable/testing.nux lib/testing.nux" disk2.img || echo "Failed testing.nux"
debugfs -w -R "write nux_portable/log.nux lib/log.nux" disk2.img || echo "Failed log.nux"
debugfs -w -R "write nux_portable/test_mem_limit.nux test_mem.nux" disk2.img || echo "Failed test_mem"
debugfs -w -R "write nux_portable/test_gc.nux test_gc.nux" disk2.img || echo "Failed test_gc"
debugfs -w -R "write nux_portable/util.nux lib/util.nux" disk2.img || echo "Failed util.nux"

# Inject verification script
debugfs -w -R "write test_sec.nux test_sec.nux" disk2.img || echo "Failed to inject test_sec.nux"
debugfs -w -R "write min_sec.nux min_sec.nux" disk2.img || echo "Failed to inject min_sec.nux"


# Run QEMU with serial output to console
echo "Ensuring fresh build..."
./build_iso.sh
if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

if [ -e /dev/kvm ]; then
    ACCEL="-enable-kvm"
    echo "KVM Acceleration Enabled 🚀"
else
    ACCEL=""
    echo "KVM Not Found (Using Software Emulation - Slower)"
fi

echo "Starting QEMU..."
echo "Use Ctrl+A, X to exit."
qemu-system-x86_64 -M pc -m 2G -cdrom ainux.iso -boot d -serial stdio -drive file=disk2.img,format=raw,index=0,media=disk $ACCEL -net nic,model=rtl8139 -net user
