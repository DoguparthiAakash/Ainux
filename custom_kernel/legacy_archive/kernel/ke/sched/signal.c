#include "signal.h"
#include "sched.h"
#include "mm/heap.h"
#include "../libc/string.h"

extern void kprint(const char *msg);

/* Default signal actions */
typedef enum {
    SIG_ACTION_TERM,      /* Terminate process */
    SIG_ACTION_CORE,      /* Terminate + core dump */
    SIG_ACTION_IGNORE,    /* Ignore signal */
    SIG_ACTION_STOP,      /* Stop process */
    SIG_ACTION_CONT       /* Continue stopped process */
} default_action_t;

/* Default actions for each signal */
static const default_action_t default_actions[NSIG] = {
    [0] = SIG_ACTION_IGNORE,
    [SIGHUP] = SIG_ACTION_TERM,
    [SIGINT] = SIG_ACTION_TERM,
    [SIGQUIT] = SIG_ACTION_CORE,
    [SIGILL] = SIG_ACTION_CORE,
    [SIGTRAP] = SIG_ACTION_CORE,
    [SIGABRT] = SIG_ACTION_CORE,
    [SIGBUS] = SIG_ACTION_CORE,
    [SIGFPE] = SIG_ACTION_CORE,
    [SIGKILL] = SIG_ACTION_TERM,
    [SIGUSR1] = SIG_ACTION_TERM,
    [SIGSEGV] = SIG_ACTION_CORE,
    [SIGUSR2] = SIG_ACTION_TERM,
    [SIGPIPE] = SIG_ACTION_TERM,
    [SIGALRM] = SIG_ACTION_TERM,
    [SIGTERM] = SIG_ACTION_TERM,
    [SIGSTKFLT] = SIG_ACTION_TERM,
    [SIGCHLD] = SIG_ACTION_IGNORE,
    [SIGCONT] = SIG_ACTION_CONT,
    [SIGSTOP] = SIG_ACTION_STOP,
    [SIGTSTP] = SIG_ACTION_STOP,
    [SIGTTIN] = SIG_ACTION_STOP,
    [SIGTTOU] = SIG_ACTION_STOP,
    [SIGURG] = SIG_ACTION_IGNORE,
    [SIGXCPU] = SIG_ACTION_CORE,
    [SIGXFSZ] = SIG_ACTION_CORE,
    [SIGVTALRM] = SIG_ACTION_TERM,
    [SIGPROF] = SIG_ACTION_TERM,
    [SIGWINCH] = SIG_ACTION_IGNORE,
    [SIGIO] = SIG_ACTION_TERM,
    [SIGPWR] = SIG_ACTION_TERM,
    [SIGSYS] = SIG_ACTION_CORE
};

void signal_init(void) {
    kprint("[SIGNAL] Subsystem initialized\n");
}

signal_pending_t *signal_alloc(void) {
    signal_pending_t *sp = (signal_pending_t *)kmalloc(sizeof(signal_pending_t));
    if (!sp) return 0;
    
    memset(sp, 0, sizeof(signal_pending_t));
    
    /* Initialize all handlers to default */
    for (int i = 0; i < NSIG; i++) {
        sp->actions[i].sa_handler = SIG_DFL;
        sp->actions[i].sa_flags = 0;
        sigemptyset(&sp->actions[i].sa_mask);
    }
    
    return sp;
}

void signal_free(signal_pending_t *sp) {
    if (sp) {
        kfree(sp);
    }
}

/* Get signal state for current task */
signal_pending_t *signal_get_current(void) {
    /* TODO: Integrate with task_struct when signals are added there */
    /* For now, return NULL - caller should allocate if needed */
    return 0;
}

sighandler_t signal_set_handler(int sig, sighandler_t handler) {
    if (sig < 1 || sig >= NSIG) return SIG_ERR;
    
    /* SIGKILL and SIGSTOP cannot be caught */
    if (sig == SIGKILL || sig == SIGSTOP) return SIG_ERR;
    
    signal_pending_t *sp = signal_get_current();
    if (!sp) return SIG_ERR;
    
    sighandler_t old = sp->actions[sig].sa_handler;
    sp->actions[sig].sa_handler = handler;
    sp->actions[sig].sa_flags = 0;
    sigemptyset(&sp->actions[sig].sa_mask);
    
    return old;
}

int signal_action(int sig, const sigaction_t *act, sigaction_t *oldact) {
    if (sig < 1 || sig >= NSIG) return -1;
    if (sig == SIGKILL || sig == SIGSTOP) return -1;
    
    signal_pending_t *sp = signal_get_current();
    if (!sp) return -1;
    
    if (oldact) {
        memcpy(oldact, &sp->actions[sig], sizeof(sigaction_t));
    }
    
    if (act) {
        memcpy(&sp->actions[sig], act, sizeof(sigaction_t));
    }
    
    return 0;
}

int signal_send(int pid, int sig) {
    if (sig < 0 || sig >= NSIG) return -1;
    if (sig == 0) return 0; /* Check if process exists */
    
    /* TODO: Find task by PID and set pending signal */
    /* For now, just a stub */
    kprint("[SIGNAL] Signal sent (stub)\n");
    
    return 0;
}

/* Deliver pending signals to current process */
void signal_deliver(void) {
    signal_pending_t *sp = signal_get_current();
    if (!sp) return;
    
    /* Find first pending, non-blocked signal */
    sigset_t deliverable = sp->pending & ~sp->blocked;
    
    if (!deliverable) return;
    
    for (int sig = 1; sig < NSIG; sig++) {
        if (!sigismember(&deliverable, sig)) continue;
        
        /* Clear pending bit */
        sigdelset(&sp->pending, sig);
        
        sighandler_t handler = sp->actions[sig].sa_handler;
        
        if (handler == SIG_IGN) {
            continue; /* Ignore */
        }
        
        if (handler == SIG_DFL) {
            /* Execute default action */
            switch (default_actions[sig]) {
                case SIG_ACTION_TERM:
                case SIG_ACTION_CORE:
                    kprint("[SIGNAL] Process terminated by signal\n");
                    /* TODO: Actually terminate the process */
                    break;
                case SIG_ACTION_STOP:
                    kprint("[SIGNAL] Process stopped by signal\n");
                    /* TODO: Set task state to STOPPED */
                    break;
                case SIG_ACTION_CONT:
                    kprint("[SIGNAL] Process continued by signal\n");
                    /* TODO: Set task state to READY if stopped */
                    break;
                case SIG_ACTION_IGNORE:
                    /* Do nothing */
                    break;
            }
        } else {
            /* Call user handler */
            /* TODO: Set up user stack frame and jump to handler */
            /* This requires careful register save/restore */
            kprint("[SIGNAL] User handler called (stub)\n");
            
            /* Block additional signals during handler */
            sigset_t old_blocked = sp->blocked;
            sp->blocked |= sp->actions[sig].sa_mask;
            if (!(sp->actions[sig].sa_flags & SA_NODEFER)) {
                sigaddset(&sp->blocked, sig);
            }
            
            /* TODO: Actually call the handler in user mode */
            /* handler(sig); */
            
            /* Restore signal mask */
            sp->blocked = old_blocked;
            
            /* Reset handler if SA_RESETHAND */
            if (sp->actions[sig].sa_flags & SA_RESETHAND) {
                sp->actions[sig].sa_handler = SIG_DFL;
            }
        }
        
        /* Only deliver one signal per call */
        break;
    }
}

int signal_procmask(int how, const sigset_t *set, sigset_t *oldset) {
    signal_pending_t *sp = signal_get_current();
    if (!sp) return -1;
    
    if (oldset) {
        *oldset = sp->blocked;
    }
    
    if (set) {
        switch (how) {
            case 0: /* SIG_BLOCK */
                sp->blocked |= *set;
                break;
            case 1: /* SIG_UNBLOCK */
                sp->blocked &= ~(*set);
                break;
            case 2: /* SIG_SETMASK */
                sp->blocked = *set;
                break;
            default:
                return -1;
        }
        
        /* Cannot block SIGKILL or SIGSTOP */
        sigdelset(&sp->blocked, SIGKILL);
        sigdelset(&sp->blocked, SIGSTOP);
    }
    
    return 0;
}

int signal_pending_check(sigset_t *set) {
    signal_pending_t *sp = signal_get_current();
    if (!sp || !set) return -1;
    
    *set = sp->pending & ~sp->blocked;
    return 0;
}

int signal_wait(const sigset_t *set, siginfo_t *info) {
    (void)set;
    (void)info;
    /* TODO: Block until one of the signals in set is delivered */
    kprint("[SIGNAL] signal_wait not implemented\n");
    return -1;
}
