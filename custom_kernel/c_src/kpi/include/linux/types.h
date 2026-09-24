/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — Linux type compatibility header
 * Provides the exact integer typedefs and primitive types that Linux drivers
 * expect. These must match the Linux ABI exactly — no guessing allowed.
 */
#ifndef _LINUX_TYPES_H
#define _LINUX_TYPES_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

/* Fixed-width Linux types — exactly mirroring linux/types.h */
typedef uint8_t   __u8;
typedef uint16_t  __u16;
typedef uint32_t  __u32;
typedef uint64_t  __u64;

typedef int8_t    __s8;
typedef int16_t   __s16;
typedef int32_t   __s32;
typedef int64_t   __s64;

typedef uint8_t   u8;
typedef uint16_t  u16;
typedef uint32_t  u32;
typedef uint64_t  u64;

typedef int8_t    s8;
typedef int16_t   s16;
typedef int32_t   s32;
typedef int64_t   s64;

typedef unsigned long   ulong;
typedef long            loff_t;
typedef unsigned long   pgoff_t;
typedef unsigned long   dma_addr_t;
typedef unsigned long   phys_addr_t;
typedef unsigned long   resource_size_t;
typedef unsigned int    gfp_t;
typedef unsigned int    fmode_t;
typedef int             atomic_t;

/* page size */
#define PAGE_SIZE       4096UL
#define PAGE_SHIFT      12
#define PAGE_MASK       (~(PAGE_SIZE - 1))

/* NULL already defined by stddef.h */

/* Linux __force / __iomem / __user annotations — no-ops in kernel space */
#define __force
#define __iomem
#define __user
#define __must_check
#define __packed        __attribute__((__packed__))
#define __aligned(x)    __attribute__((__aligned__(x)))
#define __always_inline __attribute__((__always_inline__)) inline
#define __noinline      __attribute__((__noinline__))
#define __pure          __attribute__((__pure__))
#define __cold          __attribute__((__cold__))

/* Atomic operations — single-CPU bare metal, all "atomic" ops are plain reads/writes */
static __always_inline int  atomic_read(const atomic_t *v)    { return *v; }
static __always_inline void atomic_set(atomic_t *v, int i)    { *v = i; }
static __always_inline void atomic_inc(atomic_t *v)           { (*v)++; }
static __always_inline void atomic_dec(atomic_t *v)           { (*v)--; }
static __always_inline int  atomic_dec_and_test(atomic_t *v)  { return --(*v) == 0; }
static __always_inline int  atomic_inc_and_test(atomic_t *v)  { return ++(*v) == 0; }
static __always_inline int  atomic_add_return(int i, atomic_t *v) { *v += i; return *v; }
static __always_inline int  atomic_sub_return(int i, atomic_t *v) { *v -= i; return *v; }

/* Bitwise endian helpers */
static __always_inline __u16 __swab16(__u16 x) { return __builtin_bswap16(x); }
static __always_inline __u32 __swab32(__u32 x) { return __builtin_bswap32(x); }
static __always_inline __u64 __swab64(__u64 x) { return __builtin_bswap64(x); }

/* x86-64 is always little-endian */
#define cpu_to_le16(x)  ((__u16)(x))
#define cpu_to_le32(x)  ((__u32)(x))
#define cpu_to_le64(x)  ((__u64)(x))
#define le16_to_cpu(x)  ((__u16)(x))
#define le32_to_cpu(x)  ((__u32)(x))
#define le64_to_cpu(x)  ((__u64)(x))
#define cpu_to_be16(x)  __swab16(x)
#define cpu_to_be32(x)  __swab32(x)
#define cpu_to_be64(x)  __swab64(x)
#define be16_to_cpu(x)  __swab16(x)
#define be32_to_cpu(x)  __swab32(x)
#define be64_to_cpu(x)  __swab64(x)

#endif /* _LINUX_TYPES_H */
