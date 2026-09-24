/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/slab.h
 * Memory allocation interface forwarded to Ainux's Rust heap allocator.
 * All functions are implemented in core/kpi/kpi.rs via extern "C".
 */
#ifndef _LINUX_SLAB_H
#define _LINUX_SLAB_H

#include <linux/types.h>

/* GFP flags — subset used by drivers. We treat all as equivalent. */
#define GFP_KERNEL      0x0001u
#define GFP_ATOMIC      0x0002u   /* IRQ-safe allocation */
#define GFP_NOWAIT      0x0004u
#define GFP_DMA         0x0008u
#define GFP_DMA32       0x0010u
#define GFP_ZERO        0x8000u   /* zero the memory */
#define __GFP_ZERO      GFP_ZERO
#define __GFP_NOFAIL    0x0020u   /* must not fail — we try our best */
#define __GFP_NOWARN    0x0040u

/* Core allocators — implemented in Rust */
extern void *kpi_kmalloc(size_t size, gfp_t flags);
extern void  kpi_kfree(const void *ptr);
extern void *kpi_krealloc(const void *ptr, size_t new_size, gfp_t flags);
extern void *kpi_kzalloc(size_t size, gfp_t flags);
extern void *kpi_kcalloc(size_t n, size_t size, gfp_t flags);
extern void *kpi_kmalloc_array(size_t n, size_t size, gfp_t flags);

static __always_inline void *kmalloc(size_t size, gfp_t flags)
{
    return kpi_kmalloc(size, flags);
}

static __always_inline void kfree(const void *ptr)
{
    kpi_kfree(ptr);
}

static __always_inline void *krealloc(const void *ptr, size_t new_size, gfp_t flags)
{
    return kpi_krealloc(ptr, new_size, flags);
}

static __always_inline void *kzalloc(size_t size, gfp_t flags)
{
    return kpi_kzalloc(size, flags);
}

static __always_inline void *kcalloc(size_t n, size_t size, gfp_t flags)
{
    return kpi_kcalloc(n, size, flags);
}

static __always_inline void *kmalloc_array(size_t n, size_t size, gfp_t flags)
{
    return kpi_kmalloc_array(n, size, flags);
}

/* kstrdup — used by some drivers for string duplication */
extern char *kpi_kstrdup(const char *s, gfp_t flags);
static __always_inline char *kstrdup(const char *s, gfp_t flags)
{
    return kpi_kstrdup(s, flags);
}

/* vmalloc / vfree — for large virtually-contiguous allocations */
extern void *kpi_vmalloc(size_t size);
extern void  kpi_vfree(const void *ptr);
static __always_inline void *vmalloc(size_t size)   { return kpi_vmalloc(size); }
static __always_inline void  vfree(const void *ptr) { kpi_vfree(ptr); }

/* kvmalloc — prefers kmalloc, falls back to vmalloc for large sizes */
static __always_inline void *kvmalloc(size_t size, gfp_t flags) { return kpi_kmalloc(size, flags); }
static __always_inline void  kvfree(const void *ptr)             { kpi_kfree(ptr); }

#endif /* _LINUX_SLAB_H */
