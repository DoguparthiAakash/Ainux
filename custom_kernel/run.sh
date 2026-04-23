#!/bin/bash
set -ex

# Ensure common paths are included (especially for snap and rustup in WSL)
export PATH="$PATH:/snap/bin:$HOME/.cargo/bin"

# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
nasm -f elf64 src/asm/boot.asm -o src/asm/boot.o
nasm -f bin src/asm/ap_trampoline.asm -o src/asm/ap_trampoline.bin

# Compile C/Zig modules not handled by build.rs
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic
zig build-obj src/c/tui.zig -target x86_64-freestanding-none -mcmodel=kernel -O ReleaseFast -femit-bin=src/c/tui.o

# Build Kernel
echo "Building Kernel..."
# LINK_ARGS: Only include objects NOT compiled by Cargo's build.rs
LINK_ARGS="-C link-arg=-Tlinker.ld -C link-arg=src/asm/boot.o -C link-arg=src/asm/utils.o -C link-arg=src/c/hardware.o -C link-arg=src/c/tui.o"

CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="$LINK_ARGS" cargo build --release --target x86_64-unknown-none || { echo "Cargo build failed"; exit 1; }

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
debugfs -w -R "rm hello.txt" disk2.img || true
debugfs -w -R "write hello.txt hello.txt" disk2.img || echo "debugfs failed (optional)"

# Build and Inject Userspace Hello
echo "Building hello.c..."
gcc -static -nostdlib -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables -Ttext=0x400000 -e _start src/c/hello.c -o hello.elf
debugfs -w -R "rm hello.elf" disk2.img || true
debugfs -w -R "write hello.elf hello.elf" disk2.img || echo "debugfs (hello.elf) failed"

# Hybrid Nux-LLVM Compiler Step
echo "Building test.nux using LLVM Hybrid Compiler..."
python3 tools/nux_llvm.py tools/test.nux tools/test.ll
if command -v clang >/dev/null 2>&1; then
    clang -target x86_64-unknown-none-elf -nostdlib -fno-pic -fPIE -O3 tools/test.ll -o test.elf
    debugfs -w -R "rm test.elf" disk2.img || true
    debugfs -w -R "write test.elf test.elf" disk2.img || echo "debugfs (test.elf) failed"
else
    echo "Warning: Clang not installed. Skipping LLVM backend compilation step."
    echo "Run 'sudo apt-get install clang llvm' inside WSL to enable the hybrid compiler."
fi

# Export for external VMs
if command -v qemu-img >/dev/null 2>&1; then
    echo "Exporting VM-compatible disks..."
    rm -f ainux_disk.vdi ainux_disk.vmdk
    qemu-img convert -f raw -O vdi disk2.img ainux_disk.vdi
    qemu-img convert -f raw -O vmdk disk2.img ainux_disk.vmdk
    echo "Exports ready: ainux_disk.vdi (VirtualBox), ainux_disk.vmdk (VMware)"
else
    echo "Warning: qemu-img not found. Skipping VM disk export."
fi

# Detect KVM
if [ -e /dev/kvm ]; then
    ACCEL="-enable-kvm"
    echo "KVM Acceleration Enabled ðŸš€"
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
