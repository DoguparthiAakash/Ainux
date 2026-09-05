#ifndef SPINLOCK_H
#define SPINLOCK_H

#include <stdint.h>

/* Advanced "Brutal" Lock Validator Settings */
#define DEBUG_LOCKS 1

typedef struct {
    volatile uint32_t locked; // 0 = Unlocked, 1 = Locked
    
#ifdef DEBUG_LOCKS
    const char *name;
    uint64_t holder_pid; // PID of the task holding the lock
    uint64_t acquire_addr; // Return address of acquirer
    const char *file;
    int line;
    uint32_t cpu_id; // For SMP later
#endif

} spinlock_t;

/* Constants */
#define SPINLOCK_INIT_UNLOCKED { .locked = 0 }

/* Prototypes */
void spinlock_init(spinlock_t *lock, const char *name);
void spinlock_acquire_real(spinlock_t *lock, const char *file, int line);
void spinlock_release_real(spinlock_t *lock, const char *file, int line);

/* Macros to capture caller info */
#define spinlock_acquire(lock) spinlock_acquire_real(lock, __FILE__, __LINE__)
#define spinlock_release(lock) spinlock_release_real(lock, __FILE__, __LINE__)

/* Interrupt Helpers */
static inline void disable_interrupts(void) {
    __asm__ volatile("cli");
}

static inline void enable_interrupts(void) {
    __asm__ volatile("sti");
}

static inline uint64_t __read_rflags(void) {
    uint64_t rflags;
    __asm__ volatile("pushfq; pop %0" : "=r"(rflags));
    return rflags;
}

/* Saves interrupts state and disables them. Returns RFLAGS. */
#define spinlock_irq_save(flags) \
    do { flags = __read_rflags(); disable_interrupts(); } while(0)

/* Restores interrupts state */
#define spinlock_irq_restore(flags) \
    do { if (flags & 0x200) enable_interrupts(); } while(0)

#endif
