#ifndef SIGNAL_H
#define SIGNAL_H

#include <stdint.h>

/* Standard POSIX Signals */
#define SIGHUP     1   /* Hangup */
#define SIGINT     2   /* Interrupt (Ctrl+C) */
#define SIGQUIT    3   /* Quit */
#define SIGILL     4   /* Illegal instruction */
#define SIGTRAP    5   /* Trace/breakpoint trap */
#define SIGABRT    6   /* Abort */
#define SIGBUS     7   /* Bus error */
#define SIGFPE     8   /* Floating point exception */
#define SIGKILL    9   /* Kill (cannot be caught) */
#define SIGUSR1   10   /* User-defined signal 1 */
#define SIGSEGV   11   /* Segmentation violation */
#define SIGUSR2   12   /* User-defined signal 2 */
#define SIGPIPE   13   /* Broken pipe */
#define SIGALRM   14   /* Alarm clock */
#define SIGTERM   15   /* Termination */
#define SIGSTKFLT 16   /* Stack fault */
#define SIGCHLD   17   /* Child stopped or terminated */
#define SIGCONT   18   /* Continue if stopped */
#define SIGSTOP   19   /* Stop (cannot be caught) */
#define SIGTSTP   20   /* Stop from terminal */
#define SIGTTIN   21   /* Background read from terminal */
#define SIGTTOU   22   /* Background write to terminal */
#define SIGURG    23   /* Urgent data on socket */
#define SIGXCPU   24   /* CPU time limit exceeded */
#define SIGXFSZ   25   /* File size limit exceeded */
#define SIGVTALRM 26   /* Virtual timer expired */
#define SIGPROF   27   /* Profiling timer expired */
#define SIGWINCH  28   /* Window size changed */
#define SIGIO     29   /* I/O now possible */
#define SIGPWR    30   /* Power failure */
#define SIGSYS    31   /* Bad system call */

#define NSIG      32   /* Number of signals */

/* Signal action flags */
#define SA_NOCLDSTOP  0x0001  /* Don't send SIGCHLD when children stop */
#define SA_NOCLDWAIT  0x0002  /* Don't create zombie on child death */
#define SA_SIGINFO    0x0004  /* Extended signal info */
#define SA_ONSTACK    0x0008  /* Use alternate signal stack */
#define SA_RESTART    0x0010  /* Restart syscalls if possible */
#define SA_NODEFER    0x0040  /* Don't automatically block the signal */
#define SA_RESETHAND  0x0080  /* Reset handler to SIG_DFL after execution */

/* Special signal handlers */
#define SIG_DFL  ((sighandler_t)0)   /* Default signal action */
#define SIG_IGN  ((sighandler_t)1)   /* Ignore signal */
#define SIG_ERR  ((sighandler_t)-1)  /* Error return */

/* Signal handler type */
typedef void (*sighandler_t)(int);

/* Extended signal handler type */
typedef void (*sigaction_handler_t)(int, void *, void *);

/* Signal set type (bitmask) */
typedef uint64_t sigset_t;

/* Signal info structure */
typedef struct siginfo {
    int si_signo;      /* Signal number */
    int si_errno;      /* Error number */
    int si_code;       /* Signal code */
    int si_pid;        /* Sending process ID */
    int si_uid;        /* Sending user ID */
    int si_status;     /* Exit value or signal */
    void *si_addr;     /* Fault address */
} siginfo_t;

/* Signal action structure */
typedef struct sigaction {
    union {
        sighandler_t sa_handler;           /* Signal handler */
        sigaction_handler_t sa_sigaction;  /* Extended handler */
    };
    sigset_t sa_mask;     /* Signals to block during handler */
    int sa_flags;         /* Flags */
} sigaction_t;

/* Signal pending structure (per-process) */
typedef struct signal_pending {
    sigset_t pending;        /* Pending signals bitmask */
    sigset_t blocked;        /* Blocked signals bitmask */
    sigaction_t actions[NSIG]; /* Signal handlers */
} signal_pending_t;

/* Signal Operations */

/* Initialize signal subsystem */
void signal_init(void);

/* Send signal to a process */
int signal_send(int pid, int sig);

/* Set signal handler */
sighandler_t signal_set_handler(int sig, sighandler_t handler);

/* Set signal action (more control) */
int signal_action(int sig, const sigaction_t *act, sigaction_t *oldact);

/* Block/unblock signals */
int signal_procmask(int how, const sigset_t *set, sigset_t *oldset);

/* Check for pending signals */
int signal_pending_check(sigset_t *set);

/* Wait for signal */
int signal_wait(const sigset_t *set, siginfo_t *info);

/* Deliver pending signals (called during context switch) */
void signal_deliver(void);

/* Helper macros for signal sets */
#define sigemptyset(set)     (*(set) = 0)
#define sigfillset(set)      (*(set) = ~(sigset_t)0)
#define sigaddset(set, sig)  (*(set) |= (1ULL << ((sig) - 1)))
#define sigdelset(set, sig)  (*(set) &= ~(1ULL << ((sig) - 1)))
#define sigismember(set, sig) ((*(set) >> ((sig) - 1)) & 1)

/* Signal pending for current process */
signal_pending_t *signal_get_current(void);

/* Allocate signal state for new process */
signal_pending_t *signal_alloc(void);

/* Free signal state */
void signal_free(signal_pending_t *sp);

#endif
