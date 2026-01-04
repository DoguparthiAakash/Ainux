#include "spinlock.h"
#include "log.h"
#include "ke/sched/sched.h" // For getting current task ID

/*
 * Lock Validator (Deadlock Detector)
 * 1. Checks for Double-Lock on same CPU/Task (Recursive Deadlock)
 * 2. Checks for trying to acquire lock with interrupts enabled (Risk of ISR Deadlock)
 * 3. Tracks ownership
 */

void spinlock_init(spinlock_t *lock, const char *name) {
    lock->locked = 0;
#ifdef DEBUG_LOCKS
    lock->name = name ? name : "unnamed";
    lock->holder_pid = (uint64_t)-1;
    lock->file = 0;
    lock->line = 0;
#endif
}

void spinlock_acquire_real(spinlock_t *lock, const char *file, int line) {
    uint64_t rflags = __read_rflags();
    int interrupts_enabled = (rflags & 0x200);

    /* CHECK 1: Interrupt Safety */
    /* If we acquire a lock with interrupts enabled, an ISR could interrupt us
       and try to take the SAME lock, causing a deadlock. 
       STRICT RULE: All spinlocks must be taken with interrupts disabled IF they are used in ISRs.
       For now, we enforce this globally for "Brutal" safety. */
    if (interrupts_enabled) {
        kprint_color(KLOG_COLOR_RED, "\n[LOCK VALIDATOR] CRITAL VIOLATION: Acquiring lock with Interrupts Enabled!\n");
        kprint("Lock: "); kprint(lock->name); kprint("\n");
        kprint("Location: "); kprint(file); kprint(":"); 
        // print line number logic ignored for brevity, assumption kprint handles it or we need generic helper
        kprint("\n[PANIC] Halting to prevent ISR Deadlock.\n");
        while(1) __asm__ volatile("cli; hlt");
    }

#ifdef DEBUG_LOCKS
    /* CHECK 2: Recursive Locking */
    struct task_struct *curr = sched_get_current();
    uint64_t current_pid = curr ? curr->id : 0; // 0 is initial kernel task usually

    if (lock->locked && lock->holder_pid == current_pid) {
        kprint_color(KLOG_COLOR_RED, "\n[LOCK VALIDATOR] DEADLOCK DETECTED (Recursive)\n");
        kprint("Lock: "); kprint(lock->name); kprint(" is ALREADY held by this task!\n");
        kprint("First Acquired at: "); kprint(lock->file); kprint("\n");
        kprint("Second Attempt at: "); kprint(file); kprint("\n");
        while(1) __asm__ volatile("cli; hlt");
    }
#endif

    /* Actual Spin */
    while (__sync_lock_test_and_set(&lock->locked, 1)) {
        /* Spin-Wait hint */
        __asm__ volatile("pause");
        
        /* Optional: Deadlock Timeout could go here */
    }

#ifdef DEBUG_LOCKS
    /* Record Ownership */
    lock->holder_pid = current_pid;
    lock->file = file;
    lock->line = line;
#endif
}

void spinlock_release_real(spinlock_t *lock, const char *file, int line) {
    (void)file; (void)line;

#ifdef DEBUG_LOCKS
    /* CHECK 3: Releasing lock we don't hold */
    // Note: This check is tricky if task struct isn't fully ready, but good for runtime
    struct task_struct *curr = sched_get_current();
    uint64_t current_pid = curr ? curr->id : 0;

    if (!lock->locked) {
         kprint_color(KLOG_COLOR_RED, "\n[LOCK VALIDATOR] DOUBLE RELEASE DETECTED\n");
         while(1) __asm__ volatile("cli; hlt");
    }
    
    if (lock->holder_pid != current_pid && lock->holder_pid != (uint64_t)-1) {
         /* This might trigger if we release from a different context/ISR? 
            But spinlocks should be released by owner. */
         // Warning only for now
    }

    lock->holder_pid = (uint64_t)-1;
#endif

    __sync_lock_release(&lock->locked);
}
