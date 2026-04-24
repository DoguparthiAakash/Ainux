# Boot Process and Administration

This guide explains how to build, boot, and manage the Ainux system.

## 1. Building the Kernel
The system requires `nasm`, `gcc`, `zig`, and `rustc`.
Run the automated build script:
```bash
./build_only.sh
```

## 2. Bootloader Configuration
Ainux uses **GRUB2**. The configuration file `grub.cfg` must be placed in `/boot/grub/` on the ISO root.
Example `grub.cfg`:
```
set timeout=0
set default=0
menuentry "Ainux" {
    multiboot2 /boot/ainux_kernel
    boot
}
```

## 3. ISO Creation
The ISO is created using `grub-mkrescue`.
```bash
grub-mkrescue -o ainux.iso iso_root
```

## 4. Running in QEMU
Use the following command to test the kernel with networking and graphics:
```bash
qemu-system-x86_64 -M pc -smp 4 -m 2G -cdrom ainux.iso -boot d -serial stdio -display vnc=:0 -drive file=disk2.img,format=raw -net nic,model=rtl8139 -net user
```

## 5. System Administration
*   **Logging**: Access system logs via the serial console or `dmesg` (planned).
*   **Networking**: Use the `ip` command to view and configure interfaces.
*   **Processes**: Use `ps` to monitor running tasks.
