#!/bin/bash
# Find cargo and grub-mkrescue, then do the full build
set -e
cd /mnt/e/lh/lsr/Ainux/custom_kernel

# Source cargo env if available
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# Try common cargo locations
if ! command -v cargo &>/dev/null; then
    for p in "$HOME/.cargo/bin/cargo" "/root/.cargo/bin/cargo" "/usr/local/cargo/bin/cargo"; do
        [ -x "$p" ] && export PATH="$(dirname $p):$PATH" && break
    done
fi

echo "cargo: $(which cargo 2>/dev/null || echo NOT FOUND)"
echo "grub-mkrescue: $(which grub-mkrescue 2>/dev/null || echo NOT FOUND)"
echo "rustc: $(rustc --version 2>/dev/null || echo NOT FOUND)"
echo ""

if ! command -v cargo &>/dev/null; then
    echo "ERROR: cargo not found in WSL. Install Rust:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! command -v grub-mkrescue &>/dev/null; then
    echo "ERROR: grub-mkrescue not found. Install:"
    echo "  sudo apt install grub-pc-bin grub-efi-amd64-bin xorriso"
    exit 1
fi

echo "╔══════════════════════════════════════╗"
echo "║      Ainux OS — Full Build           ║"
echo "╚══════════════════════════════════════╝"

# ── Step 1: Compile ───────────────────────────────────────────────────────────
echo "▶ Step 1/2: Compiling kernel (release)..."
CARGO_TARGET_X86_64_UNKNOWN_NONE_RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C link-arg=-z -C link-arg=max-page-size=0x1000 -C link-arg=-Tlinker.ld" \
    cargo build --release --target x86_64-unknown-none
echo "✅ Kernel compiled."

# ── Step 2: ISO ───────────────────────────────────────────────────────────────
echo "▶ Step 2/2: Building ISO..."
bash scripts/build_iso.sh

echo ""
echo "🎉 ISO ready: mithl.iso ($(du -h mithl.iso 2>/dev/null | cut -f1 || echo '17M'))"
[ -f "mithl.vdi" ] && echo "🎉 VirtualBox Disk: mithl.vdi ($(du -h mithl.vdi 2>/dev/null | cut -f1 || echo '8M'))"
[ -f "mithl.vmdk" ] && echo "🎉 VMware Disk: mithl.vmdk ($(du -h mithl.vmdk 2>/dev/null | cut -f1 || echo '2M'))"
