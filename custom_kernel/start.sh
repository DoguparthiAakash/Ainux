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

# ── Windows Host QEMU Detection (Direct Host Audio & WHPX) ───────────────────
WIN_QEMU="/mnt/c/Program Files/qemu/qemu-system-x86_64.exe"
if [ -f "$WIN_QEMU" ]; then
    echo "🪟  Windows Host QEMU detected (/mnt/c/Program Files/qemu/qemu-system-x86_64.exe)"
    echo "🔊  Audio: AC97 (Host DirectSound / SDL backend) + PC Speaker"
    
    # Check WHPX accelerator
    if "$WIN_QEMU" -accel whpx -m 64M -display none -version &>/dev/null; then
        WIN_ACCEL="-accel whpx"
        echo "⚡  Acceleration: WHPX enabled"
    else
        WIN_ACCEL="-accel tcg"
        echo "⚠️   Acceleration: TCG software emulation"
    fi

    WIN_ISO="$(wslpath -w "$ISO")"
    WIN_SERIAL="$(wslpath -w "$SCRIPT_DIR/serial.log")"
    WIN_DISK_ARG=""
    if [ -f "$DISK" ]; then
        WIN_DISK_ARG="-drive file=$(wslpath -w "$DISK"),format=raw,index=0,media=disk"
    fi

    echo ""
    echo "  ╔══════════════════════════════════════╗"
    echo "  ║         Mithl OS Bootloader          ║"
    echo "  ╚══════════════════════════════════════╝"
    echo ""
    echo "  Ctrl+Alt        → Release mouse"
    echo "  Ctrl+Alt+F      → Toggle fullscreen"
    echo ""

    exec "$WIN_QEMU" \
        -name "Mithl OS" \
        -M pc,pcspk-audiodev=snd0 \
        -smp 4 \
        -m 2G \
        -cdrom "$WIN_ISO" \
        -boot d \
        $WIN_DISK_ARG \
        $WIN_ACCEL \
        -vga std \
        -audiodev sdl,id=snd0 \
        -device AC97,audiodev=snd0 \
        -serial file:"$WIN_SERIAL" \
        $NET_ARGS
fi

# ── Linux QEMU Fallback ──────────────────────────────────────────────────────
AUDIO_ARGS="-audiodev none,id=snd0 -device AC97,audiodev=snd0"
WSL_PULSE_SOCKET=""
for s in "/mnt/wslg/runtime-dir/pulse/native" "/mnt/wslg/run/user/$(id -u)/pulse/native" "/mnt/wslg/PulseServer"; do
    if [ -S "$s" ]; then
        WSL_PULSE_SOCKET="$s"
        break
    fi
done

if [ -n "$WSL_PULSE_SOCKET" ] && qemu-system-x86_64 -audiodev help 2>&1 | grep -q 'pa'; then
    export PULSE_SERVER="unix:$WSL_PULSE_SOCKET"
    export SDL_AUDIODRIVER="pulseaudio"
    if timeout 1 qemu-system-x86_64 -m 64M -audiodev pa,id=snd0 -display none 2>&1 | grep -q 'could not connect'; then
        echo "⚠️   WSLg PulseAudio server timed out. Running without host audio bridge."
        AUDIO_ARGS="-audiodev none,id=snd0 -device AC97,audiodev=snd0"
    else
        AUDIO_ARGS="-audiodev pa,id=snd0 -device AC97,audiodev=snd0"
        echo "🔊  Audio: AC97 (WSLg PulseAudio via $WSL_PULSE_SOCKET) + PC Speaker"
    fi
elif qemu-system-x86_64 -audiodev help 2>&1 | grep -q 'pa'; then
    AUDIO_ARGS="-audiodev pa,id=snd0 -device AC97,audiodev=snd0"
    echo "🔊  Audio: AC97 (PulseAudio backend) + PC Speaker"
elif qemu-system-x86_64 -audiodev help 2>&1 | grep -q 'sdl'; then
    AUDIO_ARGS="-audiodev sdl,id=snd0 -device AC97,audiodev=snd0"
    echo "🔊  Audio: AC97 (SDL backend) + PC Speaker"
fi

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
