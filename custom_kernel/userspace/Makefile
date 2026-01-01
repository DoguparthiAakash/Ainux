CC = cc
LD = ld
NASM = nasm

CFLAGS = -O2 -Wall -Wextra -ffreestanding -fno-stack-protector -fno-stack-check -fno-lto -fno-pie -m64 -march=x86-64 -mno-80387 -mno-mmx -mno-sse -mno-sse2 -mno-red-zone
LDFLAGS = -m elf_x86_64 -nostdlib -static -T linker.ld

all: shell.elf hello.elf

shell.elf: crt0.o syscalls.o shell.o
	$(LD) $(LDFLAGS) -o shell.elf crt0.o syscalls.o shell.o

hello.elf: crt0.o syscalls.o hello.o
	$(LD) $(LDFLAGS) -o hello.elf crt0.o syscalls.o hello.o

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.asm
	$(NASM) -f elf64 $< -o $@

clean:
	rm -f *.o *.elf
