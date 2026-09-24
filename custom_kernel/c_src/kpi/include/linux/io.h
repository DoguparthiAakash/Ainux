/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/io.h
 * MMIO read/write accessors for Linux drivers on x86-64.
 * On x86 these are volatile reads/writes — no bus adapter needed.
 */
#ifndef _LINUX_IO_H
#define _LINUX_IO_H

#include <linux/types.h>

/* ── Raw MMIO accessors ──────────────────────────────────────────────────── */
static __always_inline u8 readb(const volatile void __iomem *addr)
{
    return *(const volatile u8 *)addr;
}
static __always_inline u16 readw(const volatile void __iomem *addr)
{
    return *(const volatile u16 *)addr;
}
static __always_inline u32 readl(const volatile void __iomem *addr)
{
    return *(const volatile u32 *)addr;
}
static __always_inline u64 readq(const volatile void __iomem *addr)
{
    return *(const volatile u64 *)addr;
}

static __always_inline void writeb(u8 val, volatile void __iomem *addr)
{
    *(volatile u8 *)addr = val;
}
static __always_inline void writew(u16 val, volatile void __iomem *addr)
{
    *(volatile u16 *)addr = val;
}
static __always_inline void writel(u32 val, volatile void __iomem *addr)
{
    *(volatile u32 *)addr = val;
}
static __always_inline void writeq(u64 val, volatile void __iomem *addr)
{
    *(volatile u64 *)addr = val;
}

/* readl_relaxed / writel_relaxed — no memory barrier on x86 */
#define readb_relaxed   readb
#define readw_relaxed   readw
#define readl_relaxed   readl
#define readq_relaxed   readq
#define writeb_relaxed  writeb
#define writew_relaxed  writew
#define writel_relaxed  writel
#define writeq_relaxed  writeq

/* ── x86 I/O port accessors ─────────────────────────────────────────────── */
static __always_inline u8  inb(u16 port)
{
    u8 v; __asm__ volatile("inb %1, %0" : "=a"(v) : "dN"(port)); return v;
}
static __always_inline u16 inw(u16 port)
{
    u16 v; __asm__ volatile("inw %1, %0" : "=a"(v) : "dN"(port)); return v;
}
static __always_inline u32 inl(u16 port)
{
    u32 v; __asm__ volatile("inl %1, %0" : "=a"(v) : "dN"(port)); return v;
}
static __always_inline void outb(u8 v, u16 port)
{
    __asm__ volatile("outb %0, %1" :: "a"(v), "dN"(port));
}
static __always_inline void outw(u16 v, u16 port)
{
    __asm__ volatile("outw %0, %1" :: "a"(v), "dN"(port));
}
static __always_inline void outl(u32 v, u16 port)
{
    __asm__ volatile("outl %0, %1" :: "a"(v), "dN"(port));
}

/* ── ioremap — map physical MMIO into kernel virtual space ──────────────── */
/* Implemented in Rust: maps physical address into our kernel's direct-map */
extern void __iomem *kpi_ioremap(phys_addr_t phys_addr, size_t size);
extern void __iomem *kpi_ioremap_nocache(phys_addr_t phys_addr, size_t size);
extern void          kpi_iounmap(volatile void __iomem *addr);

static __always_inline void __iomem *ioremap(phys_addr_t pa, size_t sz)       { return kpi_ioremap(pa, sz); }
static __always_inline void __iomem *ioremap_nocache(phys_addr_t pa, size_t sz){ return kpi_ioremap_nocache(pa, sz); }
static __always_inline void __iomem *ioremap_wc(phys_addr_t pa, size_t sz)    { return kpi_ioremap_nocache(pa, sz); }
static __always_inline void          iounmap(volatile void __iomem *addr)      { kpi_iounmap(addr); }

/* ── Memory barriers ────────────────────────────────────────────────────── */
#define mb()     __asm__ volatile("mfence" ::: "memory")
#define rmb()    __asm__ volatile("lfence" ::: "memory")
#define wmb()    __asm__ volatile("sfence" ::: "memory")
#define smp_mb() mb()
#define smp_rmb() rmb()
#define smp_wmb() wmb()
#define dma_wmb() wmb()

#endif /* _LINUX_IO_H */
