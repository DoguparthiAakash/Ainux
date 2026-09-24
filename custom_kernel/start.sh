#!/bin/bash
# =============================================================================
#  Ainux OS — Start Script
#  Runs the pre-built ainux.iso directly in QEMU.
# =============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ISO="$SCRIPT_DIR/ainux.iso"
DISK="$SCRIPT_DIR/disk4.img"

# ── Sanity checks ────────────────────────────────────────────────────────────
if ! command -v qemu-system-x86_64 &>/dev/null; then
    echo "❌  qemu-system-x86_64 not found."
    echo "    sudo apt install qemu-system-x86"
    exit 1
fi

if [ ! -f "$ISO" ]; then
    echo "❌  ISO not found: $ISO"
    exit 1
fi

# ── KVM acceleration ─────────────────────────────────────────────────────────
if [ -e /dev/kvm ] && [ -r /dev/kvm ]; then
    ACCEL="-enable-kvm -cpu host"
    echo "✅  KVM acceleration enabled"
else
    ACCEL="-cpu qemu64"
    echo "⚠️   KVM not available — software emulation"
fi

# ── Disk (optional) ──────────────────────────────────────────────────────────
DISK_ARG=""
if [ -f "$DISK" ]; then
    DISK_ARG="-drive file=$DISK,format=raw,index=0,media=disk"
fi

echo ""
echo "  ╔══════════════════════════════════════╗"
echo "  ║         Ainux OS Bootloader          ║"
echo "  ╚══════════════════════════════════════╝"
echo ""
echo "  Ctrl+Alt        → Release mouse"
echo "  Ctrl+Alt+F      → Toggle fullscreen"
echo ""

qemu-system-x86_64 \
    -name "Ainux OS" \
    -M pc \
    -smp 4 \
    -m 2G \
    -cdrom "$ISO" \
    -boot d \
    $DISK_ARG \
    $ACCEL \
    -vga std \
    -display gtk \
    -serial file:"$SCRIPT_DIR/serial.log" \
    -net nic,model=rtl8139 \
    -net user

echo ""
echo "Ainux OS session ended."
