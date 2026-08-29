#!/bin/bash
set -e
echo "Building Ainux C Compiler (acc)..."

# Assemble tokenize.s
if command -v nasm >/dev/null 2>&1; then
    nasm -f elf64 tokenize.s -o tokenize.o
else
    # Fallback to gas if nasm not available
    gcc -c tokenize.s -o tokenize.o
fi

# Compile and link using musl-libc sysroot
gcc -static -nostdlib -fPIE -I ../../musl-libc/sysroot/include \
    ../../musl-libc/sysroot/lib/crt1.o \
    ../../musl-libc/sysroot/lib/crti.o \
    main.c compiler.c tokenize.o \
    ../../musl-libc/sysroot/lib/libc.a \
    ../../musl-libc/sysroot/lib/crtn.o \
    -o ../../disk3_root/bin/acc.elf

echo "acc.elf built successfully."
