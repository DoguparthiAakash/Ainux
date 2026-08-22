#!/bin/bash
echo "Killing QEMU processes..."
pkill -f -9 qemu-system-x86_64 || echo "No WSL QEMU process found."
taskkill.exe /F /IM qemu-system-x86_64.exe 2>/dev/null || echo "No Windows QEMU process found."
echo "Done."
