#!/bin/bash
set -ex
export PATH="$PATH:$HOME/.cargo/bin"

# Ensure common paths are included (especially for snap and rustup in WSL)


# Build C and ASM
nasm -f elf64 src/asm/utils.asm -o src/asm/utils.o
nasm -f elf64 src/asm/boot.asm -o src/asm/boot.o
nasm -f elf64 src/asm/syscall.asm -o src/asm/syscall.o
nasm -f bin src/asm/ap_trampoline.asm -o src/asm/ap_trampoline.bin

# Compile C/Zig modules not handled by build.rs
gcc -c src/c/hardware.c -o src/c/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic
# Ensure zig is in path (Check common locations)
export PATH="$PATH:/usr/local/bin:/snap/bin"
if ! command -v zig >/dev/null 2>&1; then
    echo "Warning: zig not found in PATH. Attempting to use existing tui.o..."
else
    zig build-obj src/c/tui.zig -target x86_64-freestanding-none -mcmodel=kernel -O ReleaseFast -femit-bin=src/c/tui.o
fi

# Build Kernel
echo "Building Kernel..."
# LINK_ARGS: Include required code models and object files
LINK_ARGS="-C code-model=kernel -C relocation-model=static -C link-arg=-z -C link-arg=max-page-size=0x1000 -C link-arg=-Tlinker.ld -C link-arg=src/asm/boot.o -C link-arg=src/asm/utils.o -C link-arg=src/asm/syscall.o -C link-arg=src/c/hardware.o -C link-arg=src/c/tui.o"

CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="$LINK_ARGS" cargo build --release --offline --target x86_64-unknown-none || { echo "Cargo build failed"; exit 1; }

# Build ISO
echo "Building ISO..."
chmod +x build_iso.sh
./build_iso.sh || { echo "ISO creation failed"; exit 1; }

# Create Disk Image (EXT4) if not exists
if [ ! -f disk3.img ]; then
    echo "Creating disk3.img (32MB)..."
    dd if=/dev/zero of=disk3.img bs=1M count=32
    mkfs.ext4 -O ^extents,^64bit -F disk3.img || { echo "mkfs.ext4 failed"; exit 1; }
fi

# Populate with hello.txt
echo "Hello World from Ext4!" > hello.txt
debugfs -w -R "rm hello.txt" disk3.img || true
debugfs -w -R "write hello.txt hello.txt" disk3.img || echo "debugfs failed (optional)"

# Build and Inject Userspace Musl Test
echo "Building test_musl.c..."
./musl-libc/sysroot/bin/musl-gcc -static -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables test_musl.c -o test_musl.elf
debugfs -w -R "rm test_musl.elf" disk3.img || true
debugfs -w -R "write test_musl.elf test_musl.elf" disk3.img || echo "debugfs (test_musl.elf) failed"

echo "Building test_drm.c..."
./musl-libc/sysroot/bin/musl-gcc -static -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables test_drm.c -o test_drm.elf
debugfs -w -R "rm test_drm.elf" disk3.img || true
debugfs -w -R "write test_drm.elf test_drm.elf" disk3.img || echo "debugfs (test_drm.elf) failed"

echo "Building user_space Rust binaries..."
(cd user_space && cargo build --release --offline --target x86_64-unknown-none) || { echo "User space build failed"; exit 1; }
debugfs -w -R "rm ls.elf" disk3.img || true
debugfs -w -R "write user_space/target/x86_64-unknown-none/release/ls ls.elf" disk3.img || echo "debugfs (ls) failed"
    debugfs -w -R "rm cat.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/cat cat.elf" disk3.img || echo "debugfs (cat) failed"
    debugfs -w -R "rm mkdir.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/mkdir mkdir.elf" disk3.img || echo "debugfs (mkdir) failed"
    debugfs -w -R "rm rm.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/rm rm.elf" disk3.img || echo "debugfs (rm) failed"
    debugfs -w -R "rm rmdir.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/rmdir rmdir.elf" disk3.img || echo "debugfs (rmdir) failed"
    debugfs -w -R "rm touch.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/touch touch.elf" disk3.img || echo "debugfs (touch) failed"
    debugfs -w -R "rm cp.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/cp cp.elf" disk3.img || echo "debugfs (cp) failed"
    debugfs -w -R "rm mv.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/mv mv.elf" disk3.img || echo "debugfs (mv) failed"
debugfs -w -R "rm ping.elf" disk3.img || true
debugfs -w -R "write user_space/target/x86_64-unknown-none/release/ping ping.elf" disk3.img || echo "debugfs (ping) failed"
debugfs -w -R "rm nc.elf" disk3.img || true
debugfs -w -R "write user_space/target/x86_64-unknown-none/release/nc nc.elf" disk3.img || echo "debugfs (nc) failed"
    debugfs -w -R "rm wget.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/wget wget.elf" disk3.img || echo "debugfs (wget) failed"
    debugfs -w -R "rm deskd.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/deskd deskd.elf" disk3.img || echo "debugfs (deskd) failed"
    debugfs -w -R "rm posixd.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/posixd posixd.elf" disk3.img || echo "debugfs (posixd) failed"
    debugfs -w -R "rm fsd.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/fsd fsd.elf" disk3.img || echo "debugfs (fsd) failed"
    debugfs -w -R "rm httpd.elf" disk3.img || true
    debugfs -w -R "write user_space/target/x86_64-unknown-none/release/httpd httpd.elf" disk3.img || echo "debugfs (httpd) failed"


# Hybrid Nux-LLVM Compiler Step
echo "Building test.nux using LLVM Hybrid Compiler..."
python3 tools/nux_llvm.py tools/test.nux tools/test.ll
if command -v clang >/dev/null 2>&1; then
    clang -target x86_64-unknown-none-elf -nostdlib -fno-pic -fPIE -O3 tools/test.ll -o test.elf
    debugfs -w -R "rm test.elf" disk3.img || true
    debugfs -w -R "write test.elf test.elf" disk3.img || echo "debugfs (test.elf) failed"
else
    echo "Warning: Clang not installed. Skipping LLVM backend compilation step."
    echo "Run 'sudo apt-get install clang llvm' inside WSL to enable the hybrid compiler."
fi

# Export for external VMs
if command -v qemu-img >/dev/null 2>&1; then
    echo "Exporting VM-compatible disks..."
    rm -f ainux_disk.vdi ainux_disk.vmdk
    qemu-img convert -f raw -O vdi disk3.img ainux_disk.vdi || echo "vdi export failed"
    qemu-img convert -f raw -O vmdk disk3.img ainux_disk.vmdk || echo "vmdk export failed"
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
qemu-system-x86_64 -s -M pc -smp 4 -m 10M -cdrom ainux.iso -boot d -serial file:serial.log -drive file=disk3.img,format=raw,index=0,media=disk $ACCEL -net nic,model=rtl8139 -net user -device usb-ehci,id=usb -device usb-host,vendorid=0x148f,productid=0x7601
