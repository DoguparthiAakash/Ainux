/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/kernel.h
 * Core kernel macros and helpers expected by Linux drivers.
 */
#ifndef _LINUX_KERNEL_H
#define _LINUX_KERNEL_H

#include <linux/types.h>
#include <stdarg.h>

/* ── Compiler hints ──────────────────────────────────────────────────────── */
#define likely(x)    __builtin_expect(!!(x), 1)
#define unlikely(x)  __builtin_expect(!!(x), 0)
#define barrier()    __asm__ __volatile__("" ::: "memory")

/* ── Common math macros ──────────────────────────────────────────────────── */
#define ARRAY_SIZE(arr)     (sizeof(arr) / sizeof((arr)[0]))
#define min(a, b)           ((a) < (b) ? (a) : (b))
#define max(a, b)           ((a) > (b) ? (a) : (b))
#define clamp(val, lo, hi)  (min(max(val, lo), hi))
#define abs(x)              ((x) < 0 ? -(x) : (x))
#define DIV_ROUND_UP(n, d)  (((n) + (d) - 1) / (d))
#define ALIGN(x, a)         (((x) + (a) - 1) & ~((a) - 1))
#define IS_ALIGNED(x, a)    (((x) & ((typeof(x))(a) - 1)) == 0)
#define BITS_PER_LONG       64
#define BIT(n)              (1UL << (n))
#define BIT_MASK(n)         (1UL << ((n) % BITS_PER_LONG))
#define roundup(x, y)       ((((x) + ((y) - 1)) / (y)) * (y))
#define rounddown(x, y)     ((x) / (y) * (y))

/* ── Container of / offset of ────────────────────────────────────────────── */
#define offsetof(TYPE, MEMBER)  __builtin_offsetof(TYPE, MEMBER)
#define container_of(ptr, type, member) ({                          \
    const typeof(((type *)0)->member) *__mptr = (ptr);              \
    (type *)((char *)__mptr - offsetof(type, member)); })

/* ── Error codes — subset of what Linux drivers use ─────────────────────── */
#define EPERM    1   /* Operation not permitted */
#define ENOENT   2   /* No such file or directory */
#define ENOMEM  12   /* Out of memory */
#define EBUSY   16   /* Device or resource busy */
#define EINVAL  22   /* Invalid argument */
#define ENOSPC  28   /* No space left */
#define ENODEV  19   /* No such device */
#define ETIME   62   /* Timer expired */
#define EOPNOTSUPP 95 /* Operation not supported */
#define IS_ERR_VALUE(x) unlikely((x) >= (unsigned long)-4095)
static __always_inline void *ERR_PTR(long error) { return (void *)error; }
static __always_inline long  PTR_ERR(const void *ptr) { return (long)ptr; }
static __always_inline bool  IS_ERR(const void *ptr) { return IS_ERR_VALUE((unsigned long)ptr); }
static __always_inline bool  IS_ERR_OR_NULL(const void *ptr) { return !ptr || IS_ERR(ptr); }

/* ── printk — routes to Ainux serial+VGA ────────────────────────────────── */
/* kpi_serial_write(buf, len): writes a pre-formatted buffer — implemented in kpi.rs */
extern void kpi_serial_write(const char *buf, unsigned long len);

/* Use C's own snprintf to format then dispatch to kpi_serial_write */
#include <stdarg.h>
static inline int __kpi_printf(const char *fmt, ...) {
    char __buf[512];
    va_list __ap;
    int __n;
    va_start(__ap, fmt);
    __n = __builtin_vsnprintf(__buf, sizeof(__buf), fmt, __ap);
    va_end(__ap);
    if (__n > 0) kpi_serial_write(__buf, (unsigned long)__n);
    return __n;
}
#define printk(fmt, ...)    __kpi_printf(fmt, ##__VA_ARGS__)
#define pr_err(fmt, ...)    __kpi_printf("KPI_ERR: " fmt, ##__VA_ARGS__)
#define pr_warn(fmt, ...)   __kpi_printf("KPI_WARN: " fmt, ##__VA_ARGS__)
#define pr_info(fmt, ...)   __kpi_printf("KPI_INFO: " fmt, ##__VA_ARGS__)
#define pr_debug(fmt, ...)  do {} while (0)  /* silence debug in release */
#define dev_err(dev, fmt, ...)  __kpi_printf("DEV_ERR: " fmt, ##__VA_ARGS__)
#define dev_warn(dev, fmt, ...) __kpi_printf("DEV_WARN: " fmt, ##__VA_ARGS__)
#define dev_info(dev, fmt, ...) __kpi_printf("DEV_INFO: " fmt, ##__VA_ARGS__)
#define dev_dbg(dev, fmt, ...)  do {} while (0)

/* ── Assertions / panics ─────────────────────────────────────────────────── */
extern void kpi_panic(const char *msg) __attribute__((noreturn));
#define BUG()               kpi_panic("BUG() at " __FILE__ ":" __stringify(__LINE__))
#define BUG_ON(cond)        do { if (unlikely(cond)) BUG(); } while (0)
#define WARN_ON(cond)       do { if (unlikely(cond)) printk("WARN_ON: " #cond "\n"); } while (0)
#define WARN_ON_ONCE(cond)  WARN_ON(cond)
#define BUILD_BUG_ON(e)     ((void)sizeof(char[1 - 2 * !!(e)]))

/* ── Stringification ─────────────────────────────────────────────────────── */
#define __stringify_1(x...) #x
#define __stringify(x...)   __stringify_1(x)

/* ── Module / init stubs (we do static linking, no modules) ──────────────── */
#define MODULE_AUTHOR(x)
#define MODULE_DESCRIPTION(x)
#define MODULE_LICENSE(x)
#define MODULE_VERSION(x)
#define MODULE_DEVICE_TABLE(type, name)
#define module_init(fn)
#define module_exit(fn)
#define module_param(name, type, perm)
#define MODULE_PARM_DESC(name, desc)
#define EXPORT_SYMBOL(sym)
#define EXPORT_SYMBOL_GPL(sym)

#endif /* _LINUX_KERNEL_H */
