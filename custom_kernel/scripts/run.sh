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

WIN_QEMU="/mnt/c/Program Files/qemu/qemu-system-x86_64.exe"
if [ -f "$WIN_QEMU" ]; then
    echo "🪟  Windows Host QEMU detected"
    echo "🔊  Audio: AC97 (Host DirectSound / SDL backend) + PC Speaker"
    if "$WIN_QEMU" -accel whpx -m 64M -display none -version &>/dev/null; then
        WIN_ACCEL="-accel whpx"
        echo "⚡  Acceleration: WHPX enabled"
    else
        WIN_ACCEL="-accel tcg"
        echo "⚠️   Acceleration: TCG software emulation"
    fi
    exec "$WIN_QEMU" \
        -s \
        -name "Mithl OS" \
        -M pc,pcspk-audiodev=snd0 \
        -smp 4 \
        -m 2G \
        -cdrom "$(wslpath -w ainux.iso)" \
        -boot d \
        $WIN_ACCEL \
        -vga std \
        -audiodev sdl,id=snd0 \
        -device AC97,audiodev=snd0 \
        -drive file="$(wslpath -w disk3.img)",format=raw,index=0,media=disk \
        -net nic,model=rtl8139 -net user
fi

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

echo "Starting Mithl OS in QEMU (SMP=4, 2G RAM)..."
echo "Ctrl+A, X to exit QEMU."
qemu-system-x86_64 \
    -s \
    -M pc,pcspk-audiodev=snd0 \
    -smp 4 \
    -m 2G \
    -cdrom ainux.iso \
    -boot d \
    -serial stdio \
    -drive file=disk3.img,format=raw,index=0,media=disk \
    $ACCEL \
    -net nic,model=rtl8139 \
    -net user \
    $AUDIO_ARGS
