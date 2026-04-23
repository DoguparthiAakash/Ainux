#!/bin/bash
# Ainux Instant Start Script
# Runs the VM using pre-built ISO and Disk Image without recompiling.

# Detect KVM Acceleration
if [ -e /dev/kvm ]; then
    ACCEL="-enable-kvm"
    echo "KVM Acceleration Enabled ðŸš€"
else
    ACCEL=""
    echo "KVM Not Found (Using Software Emulation - Slower)"
fi

echo "Starting QEMU (SMP=4, 2GB RAM)..."
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
