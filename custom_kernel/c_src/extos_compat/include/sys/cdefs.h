#ifndef _SYS_CDEFS_H_
#define _SYS_CDEFS_H_

#define __COPYRIGHT(x)
#define __RCSID(x)
#define __P(x) x
#define __dead
#define __printflike(x,y)
#define __dead2
#define __pure
#define __BEGIN_DECLS
#define __END_DECLS

static const char *__progname = "prog";
static inline void setprogname(const char *p) { __progname = p; }
static inline const char *getprogname(void) { return __progname; }

#endif
