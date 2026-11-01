#!/bin/bash
# =============================================================================
#  Mithl OS — Start Script
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

# ── Network configuration ────────────────────────────────────────────────────
if [ "$NET_MODE" = "tap" ]; then
    TAP_DEV="${TAP_IF:-tap0}"
    NET_ARGS="-net nic,model=rtl8139 -net tap,ifname=$TAP_DEV,script=no,downscript=no"
    echo "🌐  Network: TAP Bridge ($TAP_DEV)"
else
    NET_ARGS="-net nic,model=rtl8139 -net user,hostfwd=tcp::8080-:8080,hostfwd=tcp::9000-:9000"
    echo "🌐  Network: User NAT with LAN Port Forward (8080, 9000)"
fi

# ── Audio configuration ──────────────────────────────────────────────────────
AUDIO_DRIVER="none"
if qemu-system-x86_64 -audiodev help 2>&1 | grep -q 'sdl'; then
    AUDIO_DRIVER="sdl"
elif qemu-system-x86_64 -audiodev help 2>&1 | grep -q 'pa'; then
    AUDIO_DRIVER="pa"
fi
AUDIO_ARGS="-audiodev $AUDIO_DRIVER,id=snd0 -device AC97,audiodev=snd0"
echo "🔊  Audio: AC97 ($AUDIO_DRIVER backend) + PC Speaker"

echo ""
echo "  ╔══════════════════════════════════════╗"
echo "  ║         Mithl OS Bootloader          ║"
echo "  ╚══════════════════════════════════════╝"
echo ""
echo "  Ctrl+Alt        → Release mouse"
echo "  Ctrl+Alt+F      → Toggle fullscreen"
echo ""

qemu-system-x86_64 \
    -name "Mithl OS" \
    -M pc,pcspk-audiodev=snd0 \
    -smp 4 \
    -m 2G \
    -cdrom "$ISO" \
    -boot d \
    $DISK_ARG \
    $ACCEL \
    -vga std \
    -display sdl \
    -serial file:"$SCRIPT_DIR/serial.log" \
    $AUDIO_ARGS \
    $NET_ARGS

echo ""
echo "Mithl OS session ended."
