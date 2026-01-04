KERNEL_SOURCE_FILES := $(shell find kernel -name '*.c')
OBJS := $(KERNEL_SOURCE_FILES:.c=.o) kernel/hal/x86_64/boot.o kernel/hal/x86_64/syscall_entry.o

CC = cc
LD = ld

# Add new include paths for architecture layers
CFLAGS = -O3 -fno-strict-aliasing -g -Wall -Wextra -pipe -I. -Ikernel -Ikernel/include -Ikernel/hal -Ikernel/hal/x86_64 -Ikernel/ke -Ikernel/ex -Ikernel/drivers -Ikernel/libc -Ikernel/crypto -ffreestanding -fno-stack-protector -fno-stack-check -fno-lto -fPIE -m64 -march=x86-64 -mno-80387 -mno-mmx -mno-sse -mno-sse2 -mno-red-zone
LDFLAGS = -m elf_x86_64 -nostdlib -static -z max-page-size=0x1000 -T kernel/hal/x86_64/linker.ld

RUST_TARGET := x86_64-unknown-none
RUST_DIR := kernel/memory_rs
RUST_LIB := $(RUST_DIR)/target/$(RUST_TARGET)/release/libmemory_rs.a

all: limine-iso

iso: limine-iso
	cp limine.iso os.iso

limine-iso: kernel.elf initrd.tar
	mkdir -p liminedir/boot
	cp kernel.elf liminedir/boot/
	cp initrd.tar liminedir/boot/
	cp limine.conf liminedir/
	cp limine/limine-bios-cd.bin liminedir/
	cp limine/limine-bios.sys liminedir/boot/
	cp limine/limine-uefi-cd.bin liminedir/
	xorriso -as mkisofs -b limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table --efi-boot limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label liminedir -o limine.iso
	./limine/limine bios-install limine.iso
	rm -rf liminedir

kernel.elf: $(OBJS) $(RUST_LIB)
	$(LD) $(LDFLAGS) $(OBJS) $(RUST_LIB) -o $@

$(RUST_LIB):
	cd $(RUST_DIR) && cargo build --release --target $(RUST_TARGET)

initrd.tar: initrd userspace_build kernel.elf
	cp userspace/shell.elf initrd/shell.elf
	cp userspace/hello.elf initrd/hello.elf
	cp userspace/hello.elf initrd/a.out
	cp kernel.elf initrd/kernel.elf
	tar -cf initrd.tar -C initrd .

userspace_build:
	$(MAKE) -C userspace

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.asm
	nasm -f elf64 $< -o $@

clean:
	rm -f kernel.elf $(OBJS) os.iso initrd.tar
	rm -rf isodir
	cd $(RUST_DIR) && cargo clean
	$(MAKE) -C userspace clean

disk.img:
	dd if=/dev/zero of=disk.img bs=1M count=64

run: limine-iso disk.img
	qemu-system-x86_64 -cdrom limine.iso -drive file=disk.img,format=raw,if=ide,index=0,media=disk -drive file=ext2.img,format=raw,if=ide,index=1,media=disk -M pc -m 2G -serial stdio -vga std -boot d -net nic,model=rtl8139 -net user

run-uefi: limine-iso disk.img
	qemu-system-x86_64 -cdrom limine.iso -drive file=disk.img,format=raw,if=ide -M pc -m 2G -bios /usr/share/ovmf/OVMF.fd -vga std -serial stdio

run-headless: limine-iso disk.img
	qemu-system-x86_64 -cdrom limine.iso -drive file=disk.img,format=raw,if=ide -M pc -m 2G -display none -serial stdio -boot d

# Run with physical USB drive (Usage: sudo make run-usb DRIVE=/dev/sdb)
run-usb: limine-iso
	qemu-system-x86_64 -cdrom limine.iso -drive file=$(DRIVE),format=raw,if=ide -M pc -m 2G -enable-kvm -bios /usr/share/ovmf/OVMF.fd
