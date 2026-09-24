/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/interrupt.h
 * IRQ registration interface. On Ainux, IRQ handlers are registered with
 * our IDT-based system. Backed by core/kpi/kpi.rs.
 */
#ifndef _LINUX_INTERRUPT_H
#define _LINUX_INTERRUPT_H

#include <linux/types.h>

/* IRQ handler return values */
#define IRQ_NONE     0
#define IRQ_HANDLED  1
#define IRQ_WAKE_THREAD 2
typedef int irqreturn_t;

/* IRQ flags */
#define IRQF_SHARED         0x00000080  /* Allow IRQ to be shared between devices */
#define IRQF_DISABLED       0x00000020  /* Disable local IRQs while processing */
#define IRQF_TRIGGER_NONE   0x00000000
#define IRQF_TRIGGER_RISING 0x00000001
#define IRQF_TRIGGER_FALLING 0x00000002
#define IRQF_TRIGGER_HIGH   0x00000004
#define IRQF_TRIGGER_LOW    0x00000008

typedef irqreturn_t (*irq_handler_t)(int, void *);

/* IRQ registration — implemented in Rust */
extern int  kpi_request_irq(unsigned int irq, irq_handler_t handler,
                            unsigned long flags, const char *name, void *dev);
extern void kpi_free_irq(unsigned int irq, void *dev);
extern void kpi_enable_irq(unsigned int irq);
extern void kpi_disable_irq(unsigned int irq);
extern void kpi_synchronize_irq(unsigned int irq);

static __always_inline int request_irq(unsigned int irq, irq_handler_t handler,
                                       unsigned long flags, const char *name, void *dev)
{
    return kpi_request_irq(irq, handler, flags, name, dev);
}

static __always_inline void free_irq(unsigned int irq, void *dev)
{
    kpi_free_irq(irq, dev);
}

static __always_inline void enable_irq(unsigned int irq)   { kpi_enable_irq(irq); }
static __always_inline void disable_irq(unsigned int irq)  { kpi_disable_irq(irq); }
static __always_inline void synchronize_irq(unsigned int irq) { kpi_synchronize_irq(irq); }

/* Spinlock stubs — single core bare metal, no contention */
typedef struct { volatile int locked; } spinlock_t;
#define SPIN_LOCK_UNLOCKED  { .locked = 0 }
#define DEFINE_SPINLOCK(name) spinlock_t name = SPIN_LOCK_UNLOCKED
static __always_inline void spin_lock_init(spinlock_t *l) { l->locked = 0; }
static __always_inline void spin_lock(spinlock_t *l)      { (void)l; }
static __always_inline void spin_unlock(spinlock_t *l)    { (void)l; }
static __always_inline void spin_lock_irqsave(spinlock_t *l, unsigned long *f)
                                                          { (void)l; (void)f; }
static __always_inline void spin_unlock_irqrestore(spinlock_t *l, unsigned long f)
                                                          { (void)l; (void)f; }
static __always_inline void spin_lock_bh(spinlock_t *l)   { (void)l; }
static __always_inline void spin_unlock_bh(spinlock_t *l) { (void)l; }

/* Mutex stubs */
typedef struct { volatile int locked; } struct_mutex;
#define DEFINE_MUTEX(name) struct_mutex name = { .locked = 0 }
static __always_inline void mutex_init(struct_mutex *m)    { m->locked = 0; }
static __always_inline void mutex_lock(struct_mutex *m)    { (void)m; }
static __always_inline void mutex_unlock(struct_mutex *m)  { (void)m; }
static __always_inline int  mutex_trylock(struct_mutex *m) { (void)m; return 1; }
/* Linux drivers use 'struct mutex' — alias it */
typedef struct_mutex mutex;

#endif /* _LINUX_INTERRUPT_H */
