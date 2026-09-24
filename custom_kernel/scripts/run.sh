#!/bin/bash
# Ainux Quick Launcher
# Runs QEMU with the already-built disk3.img and ainux.iso
# Run build_only.sh first to build the kernel and disk image.
set -e
cd "$(dirname "$0")/.."

if [ ! -f disk3.img ]; then
    echo "disk3.img not found. Run build_only.sh first."
    exit 1
fi

if [ ! -f ainux.iso ]; then
    echo "ainux.iso not found. Run build_only.sh first."
    exit 1
fi

# Detect KVM
if [ -e /dev/kvm ]; then
    ACCEL="-enable-kvm"
    echo "KVM Acceleration Enabled 🚀"
else
    ACCEL=""
    echo "KVM Not Found (Using Software Emulation - Slower)"
fi

echo "Starting Ainux in QEMU (SMP=4, 10M RAM)..."
echo "Ctrl+A, X to exit QEMU."
qemu-system-x86_64 \
    -s \
    -M pc \
    -smp 4 \
    -m 2G \
    -cdrom ainux.iso \
    -boot d \
    -serial stdio \
    -drive file=disk3.img,format=raw,index=0,media=disk \
    $ACCEL \
    -net nic,model=rtl8139 \
    -net user \
    -device usb-ehci,id=usb \
    -device usb-host,vendorid=0x148f,productid=0x7601 \
    -device AC97
