#!/bin/bash
set -e

KERNEL_BIN="target/x86_64-unknown-none/release/ainux_kernel"
ISO_ROOT="iso_root"
ISO_OUT="ainux.iso"

# ── Verify kernel binary exists ──────────────────────────────────────────────
if [ ! -f "$KERNEL_BIN" ]; then
    echo "ERROR: Kernel binary not found at $KERNEL_BIN"
    echo "Run: cargo build --release --target x86_64-unknown-none"
    exit 1
fi

# ── Build ISO directory tree ──────────────────────────────────────────────────
echo "Creating ISO root..."
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot"
mkdir -p "$ISO_ROOT/EFI/BOOT"

# Copy kernel
cp "$KERNEL_BIN" "$ISO_ROOT/boot/ainux_kernel"
echo "Kernel copied: $ISO_ROOT/boot/ainux_kernel"

# Copy GRUB config
mkdir -p "$ISO_ROOT/boot/grub"
cp grub.cfg "$ISO_ROOT/boot/grub/grub.cfg"
echo "GRUB config copied."

# ── Secure Boot MOK Signing (Key Generation) ──────────────────────────────────
if command -v openssl >/dev/null 2>&1; then
    echo "Secure Boot tools found. Preparing MOK..."
    if [ ! -f "MOK.priv" ] || [ ! -f "MOK.cer" ]; then
        echo "Generating new Machine Owner Key (MOK)..."
        openssl req -new -x509 -newkey rsa:2048 -keyout MOK.priv -outform DER -out MOK.cer -nodes -days 3650 -subj "/CN=Ainux Sovereign OS MOK/"
    fi
    # Provide the cert in the ISO root so the user can enroll it via BIOS
    cp MOK.cer "$ISO_ROOT/MOK.cer"
    echo "Secure Boot MOK.cer copied to ISO root."
else
    echo "WARNING: openssl not found. Secure Boot MOK generation skipped."
    echo "Run 'sudo apt install openssl' to enable Secure Boot support."
fi

# ── Produce bootable ISO ──────────────────────────────────────────────────────
echo "Running grub-mkrescue to create GRUB ISO..."
grub-mkrescue -o "$ISO_OUT" "$ISO_ROOT" 2>&1

echo ""
echo "══════════════════════════════════════"
echo "  UEFI/BIOS ISO ready: $ISO_OUT"
echo "══════════════════════════════════════"
