KERNEL_SOURCE_FILES := $(shell find kernel -name '*.c')
OBJS := $(KERNEL_SOURCE_FILES:.c=.o)

CC = cc
LD = ld
NASM = nasm
OBJCOPY = objcopy

CFLAGS = -O2 -g -Wall -Wextra -pipe -I. -Ikernel -Ikernel/arch/x86_64 -Ikernel/libc -ffreestanding -fno-stack-protector -fno-stack-check -fno-lto -fPIE -m64 -march=x86-64 -mno-80387 -mno-mmx -mno-sse -mno-sse2 -mno-red-zone
LDFLAGS = -m elf_x86_64 -nostdlib -static -z max-page-size=0x1000 -T kernel/arch/x86_64/linker.ld

RUST_TARGET := x86_64-unknown-none
RUST_DIR := kernel/memory_rs
RUST_LIB := $(RUST_DIR)/target/$(RUST_TARGET)/release/libmemory_rs.a

all: disk.img

# Bootloader
boot.bin: bootloader/boot.asm
	$(NASM) -f bin $< -o $@

stage2.bin: bootloader/stage2.asm
	$(NASM) -f bin $< -o $@

# Kernel
kernel.elf: $(OBJS) $(RUST_LIB)
	$(LD) $(LDFLAGS) $(OBJS) $(RUST_LIB) -o $@

kernel.bin: kernel.elf
	$(OBJCOPY) -O binary kernel.elf kernel.bin

$(RUST_LIB):
	cd $(RUST_DIR) && cargo build --release --target $(RUST_TARGET)

initrd.tar: initrd
	tar -cf initrd.tar -C initrd .

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Disk Image
disk.img: boot.bin stage2.bin kernel.bin initrd.tar
	dd if=/dev/zero of=disk.img bs=1M count=64
	cat boot.bin stage2.bin kernel.bin > disk.raw
	dd if=disk.raw of=disk.img conv=notrunc
	rm disk.raw

clean:
	rm -f kernel.elf kernel.bin boot.bin stage2.bin $(OBJS) disk.img initrd.tar
	cd $(RUST_DIR) && cargo clean

run: disk.img
	qemu-system-x86_64 -drive file=disk.img,format=raw,if=ide -M pc -m 2G -enable-kvm -serial stdio

run-headless: disk.img
	qemu-system-x86_64 -drive file=disk.img,format=raw,if=ide -M pc -m 2G -enable-kvm -display none -serial stdio

run-usb: disk.img
	qemu-system-x86_64 -drive file=$(DRIVE),format=raw,if=ide -M pc -m 2G -enable-kvm
