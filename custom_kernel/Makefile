KERNEL_SOURCE_FILES := $(shell find kernel -name '*.c')
OBJS := $(KERNEL_SOURCE_FILES:.c=.o)

CC = cc
LD = ld

CFLAGS = -O2 -g -Wall -Wextra -pipe -I. -Ikernel -Ikernel/arch/x86_64 -Ikernel/libc -ffreestanding -fno-stack-protector -fno-stack-check -fno-lto -fPIE -m64 -march=x86-64 -mno-80387 -mno-mmx -mno-sse -mno-sse2 -mno-red-zone
LDFLAGS = -m elf_x86_64 -nostdlib -static -z max-page-size=0x1000 -T kernel/arch/x86_64/linker.ld

RUST_TARGET := x86_64-unknown-none
RUST_DIR := kernel/memory_rs
RUST_LIB := $(RUST_DIR)/target/$(RUST_TARGET)/release/libmemory_rs.a

all: os.iso

iso: os.iso

limine.h:
	curl -Lo limine.h https://github.com/limine-bootloader/limine/raw/v8.x-binary/limine.h

os.iso: kernel.elf initrd.tar
	rm -rf limine iso_root
	git clone https://github.com/limine-bootloader/limine.git --branch=v8.x-binary --depth=1
	make -C limine
	mkdir -p iso_root
	cp -v kernel.elf limine.conf initrd.tar limine/limine-bios.sys \
		limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/
	xorriso -as mkisofs -b limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o os.iso
	./limine/limine bios-install os.iso
	rm -rf iso_root

kernel.elf: $(OBJS) $(RUST_LIB)
	$(LD) $(LDFLAGS) $(OBJS) $(RUST_LIB) -o $@

$(RUST_LIB):
	cd $(RUST_DIR) && cargo build --release --target $(RUST_TARGET)

initrd.tar: initrd
	tar -cf initrd.tar -C initrd .

%.o: %.c limine.h
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f kernel.elf $(OBJS) os.iso limine.h initrd.tar
	rm -rf limine iso_root
	cd $(RUST_DIR) && cargo clean

disk.img:
	dd if=/dev/zero of=disk.img bs=1M count=64

run: os.iso disk.img
	qemu-system-x86_64 -cdrom os.iso -drive file=disk.img,format=raw,if=ide -M pc -m 2G -enable-kvm

run-uefi: os.iso disk.img
	qemu-system-x86_64 -cdrom os.iso -drive file=disk.img,format=raw,if=ide -M pc -m 2G -enable-kvm -bios /usr/share/ovmf/OVMF.fd

run-headless: os.iso disk.img
	qemu-system-x86_64 -cdrom os.iso -drive file=disk.img,format=raw,if=ide -M pc -m 2G -enable-kvm -display none -serial stdio
