#!/bin/bash
set -x
cd "$(dirname "$0")/.."
set -e
export PATH="$PATH:$HOME/.cargo/bin"

# Build C and ASM
nasm -f elf64 arch/x86_64/asm/utils.asm -o arch/x86_64/asm/utils.o
nasm -f elf64 arch/x86_64/asm/boot.asm -o arch/x86_64/asm/boot.o
nasm -f bin arch/x86_64/asm/ap_trampoline.asm -o arch/x86_64/asm/ap_trampoline.bin
gcc -c c_src/hardware.c -o c_src/hardware.o -ffreestanding -mno-red-zone -mcmodel=kernel -fno-pic
# Ensure zig is in path (Check common locations)
export PATH="$PATH:/usr/local/bin:/snap/bin"
if ! command -v zig >/dev/null 2>&1; then
    echo "Warning: zig not found in PATH. Attempting to use existing tui.o..."
else
    zig build-obj c_src/tui.zig -target x86_64-freestanding-none -mcmodel=kernel -O ReleaseFast -femit-bin=c_src/tui.o
fi

# Build Kernel
echo "Building Kernel..."
CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-z -C link-arg=max-page-size=0x1000 -C link-arg=-Tlinker.ld -C link-arg=arch/x86_64/asm/syscall.o -C link-arg=c_src/hardware.o -C link-arg=c_src/tui.o" cargo build --release --verbose --target x86_64-unknown-none || { echo "Cargo build failed"; exit 1; }

# Build ISO
echo "Building ISO..."
chmod +x scripts/build_iso.sh
./scripts/build_iso.sh || { echo "ISO creation failed"; exit 1; }

# Create Staging Directory for Disk3
echo "Preparing disk3.img contents..."
# Do not wipe disk3_root entirely so we preserve the built native compiler
mkdir -p disk3_root
mkdir -p disk3_root/bin
mkdir -p disk3_root/usr/include
mkdir -p disk3_root/usr/lib
mkdir -p disk3_root/lib

# Populate with sample sources for the built-in compiler
echo "Hello World from Ext4!" > disk3_root/hello.txt

# Sample C program: hello.c
cp c_src/hello.c disk3_root/hello.c


# Sample C program: counter.c
cat > disk3_root/counter.c << 'EOF'
int main() {
    int i = 0;
    while (i < 5) {
        puts("counting...");
        i = i + 1;
    }
    return 0;
}
EOF

# Sample C program: branch.c
cat > disk3_root/branch.c << 'EOF'
int main() {
    int x = 42;
    if (x == 42) {
        puts("x is 42!");
    } else {
        puts("x is not 42");
    }
    return 0;
}
EOF

# Sample Assembly: hello.s
cat > disk3_root/hello.s << 'EOF'
.section .data
msg:
    .ascii "Hello from Ainux Assembler!\n"
msglen = . - msg
.section .text
.global main
main:
    movq $1, %rax
    movq $1, %rdi
    leaq msg(%rip), %rsi
    movq $msglen, %rdx
    syscall
    movq $60, %rax
    xor %rdi, %rdi
    syscall
EOF

echo "Sample source files created."

# Build and Inject Userspace Musl Test
echo "Building test_musl.c..."
./musl-libc/sysroot/bin/musl-gcc -static -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables test_musl.c -o disk3_root/bin/test_musl.elf

echo "Building test_drm.c..."
./musl-libc/sysroot/bin/musl-gcc -static -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables test_drm.c -o disk3_root/bin/test_drm.elf

echo "Building test_fork.c..."
./musl-libc/sysroot/bin/musl-gcc -static -fno-pie -mno-red-zone -fno-asynchronous-unwind-tables test_progs/test_fork.c -o disk3_root/bin/test_fork.elf

echo "Building user_space Rust binaries..."
(cd user_space && RUSTFLAGS="-C relocation-model=static -C link-arg=-z -C link-arg=max-page-size=0x1000" cargo build --release --offline --target x86_64-unknown-none) || { echo "User space build failed"; exit 1; }
cp user_space/target/x86_64-unknown-none/release/ls disk3_root/bin/ls.elf
cp user_space/target/x86_64-unknown-none/release/cat disk3_root/bin/cat.elf
cp user_space/target/x86_64-unknown-none/release/ping disk3_root/bin/ping.elf
cp user_space/target/x86_64-unknown-none/release/nc disk3_root/bin/nc.elf
cp user_space/target/x86_64-unknown-none/release/ps disk3_root/bin/ps.elf
cp user_space/target/x86_64-unknown-none/release/kill disk3_root/bin/kill.elf
cp user_space/target/x86_64-unknown-none/release/wget disk3_root/bin/wget.elf
cp user_space/target/x86_64-unknown-none/release/deskd disk3_root/bin/deskd.elf
cp user_space/target/x86_64-unknown-none/release/posixd disk3_root/bin/posixd.elf
cp user_space/target/x86_64-unknown-none/release/fsd disk3_root/bin/fsd.elf
cp user_space/target/x86_64-unknown-none/release/hwtest disk3_root/bin/hwtest.elf
cp user_space/target/x86_64-unknown-none/release/grep disk3_root/bin/grep.elf
cp user_space/target/x86_64-unknown-none/release/netd disk3_root/bin/netd.elf
cp user_space/target/x86_64-unknown-none/release/terminald disk3_root/bin/terminald
cp user_space/target/x86_64-unknown-none/release/sh disk3_root/bin/sh

echo "Injecting LLVM compiler (via statically linked Zig as clang)..."
if [ -f "compilers/zig-linux-x86_64-0.11.0/zig" ]; then
    cp compilers/zig-linux-x86_64-0.11.0/zig disk3_root/bin/clang
    # Optionally symlink or copy as lld, cc, etc.
    # ln -s clang disk3_root/bin/cc
else
    echo "Warning: Zig/LLVM static binary not found in compilers/zig-linux-x86_64-0.11.0/zig"
fi

# Hybrid Nux-LLVM Compiler Step
echo "Building test.nux using LLVM Hybrid Compiler..."
python3 tools/nux_llvm.py tools/test.nux tools/test.ll
if command -v clang >/dev/null 2>&1; then
    clang -target x86_64-unknown-none-elf -nostdlib -fno-pic -fPIE -O3 tools/test.ll -o disk3_root/bin/test.elf
    echo "Building C programs (awk) using LLVM..."
    clang -target x86_64-unknown-none-elf -nostdlib -fno-pic -fPIE -O3 c_src/programs/awk.c -o disk3_root/bin/awk.elf
else
    echo "Warning: Clang not installed. Skipping LLVM backend compilation step."
    echo "Run 'sudo apt-get install clang llvm' inside WSL to enable the hybrid compiler."
fi

echo "Creating disk3.img (512MB)..."
dd if=/dev/zero of=disk3.img bs=1M count=512
mkfs.ext4 -d disk3_root -O ^extents,^64bit,^dir_index -F disk3.img || { echo "mkfs.ext4 failed"; exit 1; }


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
    echo "KVM detected"
fi
