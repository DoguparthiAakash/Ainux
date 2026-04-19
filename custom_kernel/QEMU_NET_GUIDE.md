# QEMU Networking Guide for Ainux

To test the **Real Internet** capabilities I am building, ensure your QEMU command includes a functional network backend.

### Option 1: User-Mode (Easiest)
This allows the kernel to access the internet via the host's connection without root privileges.
```bash
qemu-system-x86_64 -m 512M -drive file=ainux.iso,format=raw \
    -net nic,model=rtl8139 -net user
```

### Option 2: TAP Bridge (Highest Performance)
Better for testing PING and SSH back into the kernel. Requires root.
```bash
qemu-system-x86_64 -m 512M -drive file=ainux.iso,format=raw \
    -net nic,model=rtl8139 -net tap,ifname=tap0,script=no,downscript=no
```

### Option 3: Real WiFi Passthrough (Physical)
If you have a real Atheros USB stick (AR9271), you can pass it to the VM:
```bash
qemu-system-x86_64 ... -device usb-host,vendorid=0x0cf3,productid=0x9271
```

**Note**: I am currently using the **RTL8139 (Ethernet)** as the reliable physical bridge for "Real Internet" while keeping the WiFi TUI interactive.
