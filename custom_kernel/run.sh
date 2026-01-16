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
# ./tools/nuxc.py init.nuxi hello.anux # Use kernel compiler now

echo "Injecting Nux Scripts..."
# Create library directory
debugfs -w -R "mkdir lib" disk.img || true

# Inject Libraries
debugfs -w -R "write nux_portable/io.nux lib/io.nux" disk.img || echo "Failed to inject io.nux"
debugfs -w -R "write nux_portable/math.nux lib/math.nux" disk.img || echo "Failed to inject math.nux"
debugfs -w -R "write nux_portable/util.nux lib/util.nux" disk.img || echo "Failed to inject util.nux"
debugfs -w -R "write nux_portable/graphics.nux lib/graphics.nux" disk.img || echo "Failed to inject graphics.nux"
debugfs -w -R "write nux_portable/collections.nux lib/collections.nux" disk.img || echo "Failed to inject collections.nux"
debugfs -w -R "write nux_portable/string.nux lib/string.nux" disk.img || echo "Failed to inject string.nux"
# Create Library Structure
debugfs -w -R "mkdir lib/util" disk.img || true
debugfs -w -R "mkdir lib/io" disk.img || true
debugfs -w -R "mkdir lib/lang" disk.img || true
debugfs -w -R "mkdir lib/data" disk.img || true

# Inject Libraries (Categorized)
# IO
debugfs -w -R "write nux_portable/io.nux lib/io/console.nux" disk.img || echo "Failed io.nux"
debugfs -w -R "write nux_portable/file.nux lib/io/file.nux" disk.img || echo "Failed file.nux"
# For backward compat (import "io") -> lib/io.nux mapping?
# Compiler logic maps "io" -> "lib/io.nux".
# If I move it to "lib/io/console.nux", then user must import "io.console".
# User wanted "util.*".
# I will KEEP root aliases for now OR update them to be category roots?

# Strategy:
# lib/io.nux (Aggregate or Console) -> Keep as is for "import 'io'"
# lib/util.nux (Aggregate)
# lib/lang.nux (Aggregate)

# But user wants "call by name".
# If I put file in lib/util/math.nux -> import "util.math".

debugfs -w -R "write nux_portable/math.nux lib/util/math.nux" disk.img || echo "Failed math.nux"
debugfs -w -R "write nux_portable/io.nux lib/io.nux" disk.img || echo "Failed io.nux" 
debugfs -w -R "write nux_portable/file.nux lib/file.nux" disk.img || echo "Failed file.nux"
debugfs -w -R "write nux_portable/sys.nux lib/sys.nux" disk.img || echo "Failed sys.nux"
debugfs -w -R "write nux_portable/memory.nux lib/memory.nux" disk.img || echo "Failed memory.nux"
debugfs -w -R "write nux_portable/string.nux lib/string.nux" disk.img || echo "Failed string.nux"
debugfs -w -R "write nux_portable/collections.nux lib/collections.nux" disk.img || echo "Failed collections.nux"
debugfs -w -R "write nux_portable/time.nux lib/time.nux" disk.img || echo "Failed time.nux"
debugfs -w -R "write nux_portable/testing.nux lib/testing.nux" disk.img || echo "Failed testing.nux"
debugfs -w -R "write nux_portable/log.nux lib/log.nux" disk.img || echo "Failed log.nux"
debugfs -w -R "write nux_portable/test_mem_limit.nux test_mem.nux" disk.img || echo "Failed test_mem"
debugfs -w -R "write nux_portable/test_gc.nux test_gc.nux" disk.img || echo "Failed test_gc"
debugfs -w -R "write nux_portable/util.nux lib/util.nux" disk.img || echo "Failed util.nux"


# Create Category Aggregates (Virtual)
# We need actual files for "util.nux" if someone imports "util".
# I will create 'util.nux' later which imports 'util/math'.
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
