# L99 Kernel Net v2 — Makefile
MODULE_NAME  := l99_net_core
KDIR         ?= /lib/modules/$(shell uname -r)/build
PWD          := $(shell pwd)

obj-m        += $(MODULE_NAME).o
$(MODULE_NAME)-objs := net_core.o
ccflags-y    := -Wall -Wextra -O2

# Auto-detect physical NIC for insmod
PHY_IFACE    ?= $(shell ip route show default 2>/dev/null | awk '{print $$5; exit}')
PHY_IFACE    := $(or $(PHY_IFACE),eth0)

all:
	@echo "[L99] Building for kernel $(shell uname -r) …"
	$(MAKE) -C $(KDIR) M=$(PWD) modules
	@gcc -O2 -Wall -o l99_httpd l99_httpd.c -lpthread
	@echo "[L99] Build complete."
	@echo "[L99] Detected NIC: $(PHY_IFACE)"

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	@rm -f l99_httpd
	@echo "[L99] Clean done"

# Full internet install
internet:
	@sudo ./internet_setup.sh

# NAT only (safer)
nat:
	@sudo ./internet_setup.sh --no-bridge

status:
	@sudo ./internet_setup.sh --status

teardown:
	@sudo ./internet_setup.sh --teardown

.PHONY: all clean internet nat status teardown
