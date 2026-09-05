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
if command -v openssl >/dev/null 2>&1 && command -v sbsign >/dev/null 2>&1; then
    echo "Secure Boot tools found. Preparing MOK..."
    if [ ! -f "MOK.priv" ] || [ ! -f "MOK.cer" ]; then
        echo "Generating new Machine Owner Key (MOK)..."
        # Generate PEM format for sbsigntool
        openssl req -new -x509 -newkey rsa:2048 -keyout MOK.priv -outform PEM -out MOK.cer -nodes -days 3650 -subj "/CN=Ainux Sovereign OS MOK/"
        # Convert to DER format for UEFI BIOS enrollment
        openssl x509 -in MOK.cer -outform DER -out MOK.der
    fi
    # Provide the cert in the ISO root so the user can enroll it via BIOS
    cp MOK.der "$ISO_ROOT/MOK.der"
    echo "Secure Boot MOK.der copied to ISO root."

    # Note: We cannot sbsign ainux_kernel directly because it is an ELF multiboot kernel, not a PE/COFF EFI binary.
    # GRUB will boot it via the multiboot/multiboot2 command. If strictly required by Secure Boot, 
    # we would need GRUB's PGP detached signatures, or wrap the kernel in an EFI stub.

    # Create a standalone GRUB EFI payload and sign it
    echo "Generating and signing GRUB EFI payload for Secure Boot..."
    grub-mkimage -O x86_64-efi -o bootx64.efi -p /boot/grub boot linux ext2 fat serial part_msdos part_gpt normal efi_gop iso9660 search multiboot multiboot2
    sbsign --key MOK.priv --cert MOK.cer --output "$ISO_ROOT/EFI/BOOT/BOOTX64.EFI" bootx64.efi
else
    echo "WARNING: openssl or sbsigntool not found. Secure Boot MOK generation & signing skipped."
    echo "Run 'sudo apt install openssl sbsigntool' to enable Secure Boot support."
fi

# ── Produce bootable ISO ──────────────────────────────────────────────────────
echo "Running grub-mkrescue to create GRUB ISO..."
# We pass --uefi-secure-boot if supported by grub-mkrescue, or just rely on the injected BOOTX64.EFI
grub-mkrescue -o "$ISO_OUT" "$ISO_ROOT" 2>&1

echo ""
echo "══════════════════════════════════════"
echo "  UEFI/BIOS ISO ready: $ISO_OUT"
echo "══════════════════════════════════════"
