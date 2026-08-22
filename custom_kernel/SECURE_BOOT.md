# Secure Boot Guide for Ainux

To boot Ainux under a real or virtualized UEFI firmware with Secure Boot enabled, you must cryptographically sign the bootloader (`BOOTX64.EFI`) and the kernel (`ainux_kernel`).

Since Ainux uses a custom kernel and bootloader, you must generate your own Machine Owner Keys (MOK), enroll them into your UEFI firmware, and sign the binaries.

## 1. Install Prerequisites

On a Linux machine (or WSL), install the required tools:
```bash
sudo apt-get install efitools sbsigntool openssl
```

## 2. Generate Your Custom Keys

Generate a new private key and public certificate:
```bash
openssl req -newkey rsa:2048 -nodes -keyout ainux_MOK.key -new -x509 -sha256 -days 3650 -subj "/CN=Ainux Custom MOK/" -out ainux_MOK.crt
```

Convert the certificate to DER format (which the UEFI firmware expects):
```bash
openssl x509 -outform DER -in ainux_MOK.crt -out ainux_MOK.cer
```

## 3. Sign the Binaries

You must sign the Limine EFI executable *before* building the ISO, or extract it, sign it, and repack the ISO. 

To sign the Limine bootloader:
```bash
sbsign --key ainux_MOK.key --cert ainux_MOK.crt --output limine/BOOTX64.EFI limine/BOOTX64.EFI
```

To sign the kernel (Limine doesn't strictly verify the kernel signature unless patched, but it's good practice for an end-to-end chain of trust):
```bash
sbsign --key ainux_MOK.key --cert ainux_MOK.crt --output target/x86_64-unknown-none/release/ainux_kernel target/x86_64-unknown-none/release/ainux_kernel
```

## 4. Enroll the Key in UEFI

1. Copy the `ainux_MOK.cer` file to a FAT32 formatted USB drive.
2. Reboot your computer into the UEFI/BIOS settings.
3. Locate the **Secure Boot** configuration.
4. Set the mode to **Custom** or **Setup Mode** (if required).
5. Choose **Key Management** > **Append Key** or **Enroll Key** (varies by motherboard).
6. Select the `ainux_MOK.cer` file from your USB drive and enroll it into the `db` (Signature Database) or `MOK` list.
7. Save and exit.

## 5. Booting in QEMU with Secure Boot

To test in QEMU, you need the OVMF UEFI firmware with Secure Boot variables.

1. Install OVMF: `sudo apt-get install ovmf`
2. Copy the variable store: `cp /usr/share/OVMF/OVMF_VARS.fd my_vars.fd`
3. Launch QEMU with the OVMF code and variables, and attach a virtual FAT drive with your `.cer` file to enroll it via the virtual EFI shell, or use `virt-fw-vars` to inject the certificate offline.
