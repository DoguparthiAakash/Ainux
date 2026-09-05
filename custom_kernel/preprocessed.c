# 0 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c"
# 0 "<built-in>"
# 0 "<command-line>"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c"
# 92 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cdefs.h" 1
# 76 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cdefs.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/cdefs.h" 1
# 77 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cdefs.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cdefs_elf.h" 1
# 79 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cdefs.h" 2
# 737 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cdefs.h"
static __inline long long __zeroll(void) { return 0; }
static __inline unsigned long long __zeroull(void) { return 0; }
# 93 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
__asm(".pushsection " ".ident" ",\"MS\",@progbits,1\n" ".asciz \"" "$NetBSD: if.c,v 1.538 2026/05/14 08:05:48 roy Exp $" "\"\n" ".popsection");
# 104 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/param.h" 1



# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/types.h" 1




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/_types.h" 1
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/types.h" 2
# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 1 3 4
# 50 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 3 4
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/ansi.h" 1 3 4




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_ansi.h" 1 3 4
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_ansi.h" 3 4
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_types.h" 1 3 4




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_int_types.h" 1 3 4
# 45 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_int_types.h" 3 4

# 45 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_int_types.h" 3 4
typedef signed char __int8_t;
typedef unsigned char __uint8_t;
typedef short int __int16_t;
typedef short unsigned int __uint16_t;
typedef int __int32_t;
typedef unsigned int __uint32_t;
typedef long int __int64_t;
typedef long unsigned int __uint64_t;





typedef long int __intptr_t;
typedef long unsigned int __uintptr_t;
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_types.h" 2 3 4
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_ansi.h" 2 3 4
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/ansi.h" 2 3 4
# 51 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 2 3 4
# 145 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 3 4
typedef long int ptrdiff_t;
# 214 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 3 4
typedef long unsigned int size_t;
# 329 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 3 4
typedef int wchar_t;
# 425 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 3 4
typedef struct {
  long long __max_align_ll __attribute__((__aligned__(__alignof__(long long))));
  long double __max_align_ld __attribute__((__aligned__(__alignof__(long double))));
# 436 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 3 4
} max_align_t;
# 7 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/types.h" 2


# 8 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/types.h"
typedef unsigned char __cpu_simple_lock_nv_t;
typedef long register_t;
typedef unsigned long vaddr_t;
typedef unsigned long vsize_t;
typedef unsigned long paddr_t;
typedef unsigned long psize_t;
typedef unsigned int socklen_t;
# 46 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2





# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ansi.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ansi.h"
typedef char * __caddr_t;
typedef __uint32_t __gid_t;
typedef __uint32_t __in_addr_t;
typedef __uint16_t __in_port_t;
typedef __uint32_t __mode_t;
typedef __uint32_t __accmode_t;
typedef __int64_t __off_t;
typedef __int32_t __pid_t;
typedef __uint8_t __sa_family_t;
typedef unsigned int __socklen_t;
typedef __uint32_t __uid_t;
typedef __uint64_t __fsblkcnt_t;
typedef __uint64_t __fsfilcnt_t;

struct __tag_wctrans_t;
typedef struct __tag_wctrans_t *__wctrans_t;

struct __tag_wctype_t;
typedef struct __tag_wctype_t *__wctype_t;





typedef union {
 __int64_t __mbstateL;
 char __mbstate8[128];
} __mbstate_t;
# 73 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ansi.h"
typedef __builtin_va_list __va_list;
# 52 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2


typedef __int8_t int8_t;




typedef __uint8_t uint8_t;




typedef __int16_t int16_t;




typedef __uint16_t uint16_t;




typedef __int32_t int32_t;




typedef __uint32_t uint32_t;




typedef __int64_t int64_t;




typedef __uint64_t uint64_t;



typedef uint8_t u_int8_t;
typedef uint16_t u_int16_t;
typedef uint32_t u_int32_t;
typedef uint64_t u_int64_t;

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdint.h" 1
# 79 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdint.h"
typedef __intptr_t intptr_t;




typedef __uintptr_t uintptr_t;



# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_mwgwtypes.h" 1




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_int_mwgwtypes.h" 1
# 45 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_int_mwgwtypes.h"
typedef signed char int_least8_t;
typedef unsigned char uint_least8_t;
typedef short int int_least16_t;
typedef short unsigned int uint_least16_t;
typedef int int_least32_t;
typedef unsigned int uint_least32_t;
typedef long int int_least64_t;
typedef long unsigned int uint_least64_t;


typedef signed char int_fast8_t;
typedef unsigned char uint_fast8_t;
typedef long int int_fast16_t;
typedef long unsigned int uint_fast16_t;
typedef long int int_fast32_t;
typedef long unsigned int uint_fast32_t;
typedef long int int_fast64_t;
typedef long unsigned int uint_fast64_t;



typedef long int intmax_t;
typedef long unsigned int uintmax_t;
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_mwgwtypes.h" 2
# 89 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdint.h" 2



# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_limits.h" 1




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_int_limits.h" 1
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_limits.h" 2
# 93 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdint.h" 2




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_const.h" 1
# 98 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdint.h" 2


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/wchar_limits.h" 1




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/common_wchar_limits.h" 1
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/wchar_limits.h" 2
# 101 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdint.h" 2
# 99 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2


typedef unsigned char u_char;
typedef unsigned short u_short;
typedef unsigned int u_int;
typedef unsigned long u_long;

typedef unsigned char unchar;
typedef unsigned short ushort;
typedef unsigned int uint;
typedef unsigned long ulong;


typedef uint64_t u_quad_t;
typedef int64_t quad_t;
typedef quad_t * qaddr_t;
# 126 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
typedef int64_t longlong_t;
typedef uint64_t u_longlong_t;

typedef int64_t blkcnt_t;
typedef int32_t blksize_t;


typedef __fsblkcnt_t fsblkcnt_t;




typedef __fsfilcnt_t fsfilcnt_t;
# 154 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
typedef int64_t daddr_t;


typedef uint64_t dev_t;
typedef uint32_t fixpt_t;


typedef __gid_t gid_t;



typedef uint32_t id_t;
# 175 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
typedef uint64_t ino_t;

typedef long key_t;


typedef __mode_t mode_t;




typedef __accmode_t accmode_t;



typedef uint32_t nlink_t;


typedef __off_t off_t;




typedef __pid_t pid_t;


typedef int32_t lwpid_t;
typedef uint64_t rlim_t;
typedef int32_t segsz_t;
typedef int32_t swblk_t;


typedef __uid_t uid_t;



typedef int mqd_t;

typedef unsigned long cpuid_t;

typedef int psetid_t;

typedef volatile __cpu_simple_lock_nv_t __cpu_simple_lock_t;



# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdbool.h" 1
# 221 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2




typedef int boolean_t;
# 240 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h" 2
# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
typedef __in_addr_t in_addr_t;




typedef __in_port_t in_port_t;




# 69 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
#pragma GCC visibility push(default)
# 69 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"

uint32_t htonl(uint32_t) __attribute__((__const__));
uint16_t htons(uint16_t) __attribute__((__const__));
uint32_t ntohl(uint32_t) __attribute__((__const__));
uint16_t ntohs(uint16_t) __attribute__((__const__));

# 74 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
#pragma GCC visibility pop
# 74 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"






# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/endian_machdep.h" 1
# 81 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h" 2
# 109 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/bswap.h" 1




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h" 1
# 11 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/bswap.h" 1
# 12 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h" 2


# 13 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h"
#pragma GCC visibility push(default)
# 13 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h"



uint16_t bswap16(uint16_t) __attribute__((__const__));
uint32_t bswap32(uint32_t) __attribute__((__const__));




uint64_t bswap64(uint64_t) __attribute__((__const__));

# 23 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h"
#pragma GCC visibility pop
# 23 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/bswap.h"

# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/bswap.h" 2
# 110 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h" 2
# 207 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
static __inline void __attribute__((__unused__)) be16enc(void *dst, uint16_t u) { u = ((uint16_t)(__builtin_constant_p((((uint16_t)((u))))) ? (((uint16_t)(((((((uint16_t)((u)))) & 0xff00) >> 8) | (((((uint16_t)((u)))) & 0x00ff) << 8))))) : bswap16(((uint16_t)((u)))))); __builtin_memcpy(dst, &u, sizeof(u)); }
static __inline void __attribute__((__unused__)) be32enc(void *dst, uint32_t u) { u = ((uint32_t)(__builtin_constant_p((((uint32_t)((u))))) ? (((uint32_t)(((((((uint32_t)((u)))) & 0xff000000) >> 24) | (((((uint32_t)((u)))) & 0x00ff0000) >> 8) | (((((uint32_t)((u)))) & 0x0000ff00) << 8) | (((((uint32_t)((u)))) & 0x000000ff) << 24))))) : bswap32(((uint32_t)((u)))))); __builtin_memcpy(dst, &u, sizeof(u)); }
static __inline void __attribute__((__unused__)) be64enc(void *dst, uint64_t u) { u = ((uint64_t)(__builtin_constant_p((((uint64_t)((u))))) ? (((uint64_t)(((((((uint64_t)((u)))) & 0xff00000000000000ull) >> 56) | (((((uint64_t)((u)))) & 0x00ff000000000000ull) >> 40) | (((((uint64_t)((u)))) & 0x0000ff0000000000ull) >> 24) | (((((uint64_t)((u)))) & 0x000000ff00000000ull) >> 8) | (((((uint64_t)((u)))) & 0x00000000ff000000ull) << 8) | (((((uint64_t)((u)))) & 0x0000000000ff0000ull) << 24) | (((((uint64_t)((u)))) & 0x000000000000ff00ull) << 40) | (((((uint64_t)((u)))) & 0x00000000000000ffull) << 56))))) : bswap64(((uint64_t)((u)))))); __builtin_memcpy(dst, &u, sizeof(u)); }
static __inline void __attribute__((__unused__)) le16enc(void *dst, uint16_t u) { u = ((uint16_t)((u))); __builtin_memcpy(dst, &u, sizeof(u)); }
static __inline void __attribute__((__unused__)) le32enc(void *dst, uint32_t u) { u = ((uint32_t)((u))); __builtin_memcpy(dst, &u, sizeof(u)); }
static __inline void __attribute__((__unused__)) le64enc(void *dst, uint64_t u) { u = ((uint64_t)((u))); __builtin_memcpy(dst, &u, sizeof(u)); }
# 224 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/endian.h"
static __inline uint16_t __attribute__((__unused__)) be16dec(const void *_buf) { uint16_t u; __builtin_memcpy(&u, _buf, sizeof(u)); return ((uint16_t)(__builtin_constant_p((((uint16_t)((u))))) ? (((uint16_t)(((((((uint16_t)((u)))) & 0xff00) >> 8) | (((((uint16_t)((u)))) & 0x00ff) << 8))))) : bswap16(((uint16_t)((u)))))); }
static __inline uint32_t __attribute__((__unused__)) be32dec(const void *_buf) { uint32_t u; __builtin_memcpy(&u, _buf, sizeof(u)); return ((uint32_t)(__builtin_constant_p((((uint32_t)((u))))) ? (((uint32_t)(((((((uint32_t)((u)))) & 0xff000000) >> 24) | (((((uint32_t)((u)))) & 0x00ff0000) >> 8) | (((((uint32_t)((u)))) & 0x0000ff00) << 8) | (((((uint32_t)((u)))) & 0x000000ff) << 24))))) : bswap32(((uint32_t)((u)))))); }
static __inline uint64_t __attribute__((__unused__)) be64dec(const void *_buf) { uint64_t u; __builtin_memcpy(&u, _buf, sizeof(u)); return ((uint64_t)(__builtin_constant_p((((uint64_t)((u))))) ? (((uint64_t)(((((((uint64_t)((u)))) & 0xff00000000000000ull) >> 56) | (((((uint64_t)((u)))) & 0x00ff000000000000ull) >> 40) | (((((uint64_t)((u)))) & 0x0000ff0000000000ull) >> 24) | (((((uint64_t)((u)))) & 0x000000ff00000000ull) >> 8) | (((((uint64_t)((u)))) & 0x00000000ff000000ull) << 8) | (((((uint64_t)((u)))) & 0x0000000000ff0000ull) << 24) | (((((uint64_t)((u)))) & 0x000000000000ff00ull) << 40) | (((((uint64_t)((u)))) & 0x00000000000000ffull) << 56))))) : bswap64(((uint64_t)((u)))))); }
static __inline uint16_t __attribute__((__unused__)) le16dec(const void *_buf) { uint16_t u; __builtin_memcpy(&u, _buf, sizeof(u)); return ((uint16_t)((u))); }
static __inline uint32_t __attribute__((__unused__)) le32dec(const void *_buf) { uint32_t u; __builtin_memcpy(&u, _buf, sizeof(u)); return ((uint32_t)((u))); }
static __inline uint64_t __attribute__((__unused__)) le64dec(const void *_buf) { uint64_t u; __builtin_memcpy(&u, _buf, sizeof(u)); return ((uint64_t)((u))); }
# 241 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2
# 249 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
union __semun {
 int val;
 struct semid_ds *buf;
 unsigned short *array;
};
# 277 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
typedef int32_t __devmajor_t, __devminor_t;
# 290 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
typedef 
# 290 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 3 4
       unsigned int 
# 290 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                      clock_t;
# 306 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
typedef long int 
# 306 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                      ssize_t;




typedef 
# 311 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 3 4
       __int64_t 
# 311 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                     time_t;




typedef 
# 316 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 3 4
       int 
# 316 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                        clockid_t;




typedef 
# 321 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 3 4
       int 
# 321 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                      timer_t;




typedef 
# 326 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 3 4
       int 
# 326 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                         suseconds_t;




typedef 
# 331 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 3 4
       unsigned int 
# 331 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
                        useconds_t;




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/fd_set.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/fd_set.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 39 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/fd_set.h" 2







typedef __uint32_t __fd_mask;
# 66 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/fd_set.h"
typedef struct fd_set {
 __fd_mask fds_bits[(((256) + ((32) - 1)) / (32))];
} fd_set;
# 337 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h" 2



typedef struct kauth_cred *kauth_cred_t;

typedef int pri_t;
# 352 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/types.h"
struct lwp;
typedef struct lwp lwp_t;
struct __ucontext;
struct proc;
typedef struct proc proc_t;
struct pgrp;
struct rusage;
struct file;
typedef struct file file_t;
struct buf;
typedef struct buf buf_t;
struct tty;
struct uio;
# 5 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/param.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/inttypes.h" 1
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/inttypes.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_fmtio.h" 1




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/arch/amd64/include/int_fmtio.h" 1
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/int_fmtio.h" 2
# 44 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/inttypes.h" 2
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/param.h" 2
# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stdbool.h" 1 3 4
# 7 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/param.h" 2
# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/limits.h" 1 3 4
# 34 "/usr/lib/gcc/x86_64-linux-gnu/14/include/limits.h" 3 4
# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/syslimits.h" 1 3 4






# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/limits.h" 1 3 4
# 210 "/usr/lib/gcc/x86_64-linux-gnu/14/include/limits.h" 3 4
# 1 "/usr/include/limits.h" 1 3 4
# 26 "/usr/include/limits.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/libc-header-start.h" 1 3 4
# 33 "/usr/include/x86_64-linux-gnu/bits/libc-header-start.h" 3 4
# 1 "/usr/include/features.h" 1 3 4
# 415 "/usr/include/features.h" 3 4
# 1 "/usr/include/features-time64.h" 1 3 4
# 20 "/usr/include/features-time64.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/wordsize.h" 1 3 4
# 21 "/usr/include/features-time64.h" 2 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/timesize.h" 1 3 4
# 19 "/usr/include/x86_64-linux-gnu/bits/timesize.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/wordsize.h" 1 3 4
# 20 "/usr/include/x86_64-linux-gnu/bits/timesize.h" 2 3 4
# 22 "/usr/include/features-time64.h" 2 3 4
# 416 "/usr/include/features.h" 2 3 4
# 501 "/usr/include/features.h" 3 4
# 1 "/usr/include/stdc-predef.h" 1 3 4
# 502 "/usr/include/features.h" 2 3 4
# 547 "/usr/include/features.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/gnu/stubs.h" 1 3 4
# 10 "/usr/include/x86_64-linux-gnu/gnu/stubs.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/gnu/stubs-64.h" 1 3 4
# 11 "/usr/include/x86_64-linux-gnu/gnu/stubs.h" 2 3 4
# 548 "/usr/include/features.h" 2 3 4
# 34 "/usr/include/x86_64-linux-gnu/bits/libc-header-start.h" 2 3 4
# 27 "/usr/include/limits.h" 2 3 4
# 195 "/usr/include/limits.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/posix1_lim.h" 1 3 4
# 27 "/usr/include/x86_64-linux-gnu/bits/posix1_lim.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/wordsize.h" 1 3 4
# 28 "/usr/include/x86_64-linux-gnu/bits/posix1_lim.h" 2 3 4
# 161 "/usr/include/x86_64-linux-gnu/bits/posix1_lim.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/local_lim.h" 1 3 4
# 38 "/usr/include/x86_64-linux-gnu/bits/local_lim.h" 3 4
# 1 "/usr/include/linux/limits.h" 1 3 4
# 39 "/usr/include/x86_64-linux-gnu/bits/local_lim.h" 2 3 4
# 81 "/usr/include/x86_64-linux-gnu/bits/local_lim.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/pthread_stack_min-dynamic.h" 1 3 4
# 29 "/usr/include/x86_64-linux-gnu/bits/pthread_stack_min-dynamic.h" 3 4
# 1 "/usr/include/x86_64-linux-gnu/bits/pthread_stack_min.h" 1 3 4
# 30 "/usr/include/x86_64-linux-gnu/bits/pthread_stack_min-dynamic.h" 2 3 4
# 82 "/usr/include/x86_64-linux-gnu/bits/local_lim.h" 2 3 4
# 162 "/usr/include/x86_64-linux-gnu/bits/posix1_lim.h" 2 3 4
# 196 "/usr/include/limits.h" 2 3 4



# 1 "/usr/include/x86_64-linux-gnu/bits/posix2_lim.h" 1 3 4
# 200 "/usr/include/limits.h" 2 3 4
# 211 "/usr/lib/gcc/x86_64-linux-gnu/14/include/limits.h" 2 3 4
# 8 "/usr/lib/gcc/x86_64-linux-gnu/14/include/syslimits.h" 2 3 4
# 35 "/usr/lib/gcc/x86_64-linux-gnu/14/include/limits.h" 2 3 4
# 8 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/param.h" 2
# 18 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/param.h"
typedef void* module_t;
# 105 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/mbuf.h" 1





struct m_ext {
    char *ext_buf;
    size_t ext_size;
};


struct mbuf {
    struct mbuf *m_next;
    struct mbuf *m_nextpkt;
    char *m_data;
    int m_len;
    int m_type;
    int m_flags;
    struct m_ext m_ext;
};
# 39 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/mbuf.h"
void mbinit(void);
# 106 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/systm.h" 1




# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 1 3 4
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/systm.h" 2







int printf(const char *fmt, ...);
int strcmp(const char *s1, const char *s2);

void *malloc(size_t size);
void free(void *ptr);

struct thread { int dummy; };
extern struct thread *curthread;

int sigdeferstop(int mode);
typedef void* module_t;

typedef int (*copyin_t)(const void *uaddr, void *kaddr, size_t len);
typedef int (*copyout_t)(const void *kaddr, void *uaddr, size_t len);
void sigallowstop(int prev_stops);
# 107 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 1
# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
typedef struct callout {
 void *_c_store[10];
} callout_t;
# 105 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
struct cpu_info;

void callout_startup(void);
void callout_init_cpu(struct cpu_info *);
void callout_hardclock(void);

void callout_init(callout_t *, u_int);
void callout_destroy(callout_t *);
void callout_setfunc(callout_t *, void (*)(void *), void *);
void callout_reset(callout_t *, int, void (*)(void *), void *);
void callout_schedule(callout_t *, int);

# 116 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 3 4
_Bool 
# 116 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
    callout_stop(callout_t *);

# 117 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 3 4
_Bool 
# 117 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
    callout_halt(callout_t *, void *);

# 118 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 3 4
_Bool 
# 118 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
    callout_pending(callout_t *);

# 119 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 3 4
_Bool 
# 119 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
    callout_expired(callout_t *);

# 120 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 3 4
_Bool 
# 120 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
    callout_active(callout_t *);

# 121 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h" 3 4
_Bool 
# 121 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/callout.h"
    callout_invoking(callout_t *);
void callout_ack(callout_t *);
void callout_bind(callout_t *, struct cpu_info *);
# 108 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/proc.h" 1
# 109 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/socket.h" 1






typedef unsigned short sa_family_t;


struct sockaddr {
    unsigned char sa_len;
    sa_family_t sa_family;
    char sa_data[14];
};

struct sockaddr_storage {
    unsigned char ss_len;
    sa_family_t ss_family;
    char __ss_pad1[6];
    long __ss_align;
    char __ss_pad2[112];
};
# 110 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 1
# 66 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 39 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h" 2



# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/selinfo.h" 1
# 67 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/selinfo.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h" 1
# 34 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h" 2


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/null.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h" 2
# 66 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
struct kevent {
 uintptr_t ident;
 uint32_t filter;
 uint32_t flags;
 uint32_t fflags;
 int64_t data;
 void *udata;
 uint64_t ext[4];
};

static __inline void
_EV_SET(struct kevent *_kevp, uintptr_t _ident, uint32_t _filter,
    uint32_t _flags, uint32_t _fflags, int64_t _data, void *_udata)
{
 _kevp->ident = _ident;
 _kevp->filter = _filter;
 _kevp->flags = _flags;
 _kevp->fflags = _fflags;
 _kevp->data = _data;
 _kevp->udata = _udata;
}
# 184 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/queue.h" 1
# 185 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h" 2
struct knote;
struct klist { struct knote *slh_first; };





# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslimits.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslimits.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslimits.h" 2
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioccom.h" 1
# 44 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h" 2
# 54 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h"
struct winsize {
 unsigned short ws_row;
 unsigned short ws_col;
 unsigned short ws_xpixel;
 unsigned short ws_ypixel;
};


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h" 2
# 80 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h"
struct ptmget {
 int cfd;
 int sfd;
 char cn[1024];
 char sn[1024];
};
# 117 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ttycom.h"
typedef char linedn_t[32];
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h" 2






struct ttysize {
 unsigned short ts_lines;
 unsigned short ts_cols;
 unsigned short ts_xxx;
 unsigned short ts_yyy;
};





# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/dkio.h" 1
# 36 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/dkio.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/prop/plistref.h" 1




struct plistref {
 void *pref_plist_iovec;
};
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/dkio.h" 2
# 61 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/filio.h" 1
# 62 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sockio.h" 1
# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h" 2
# 72 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h"
struct ioctl_pt {
 unsigned long com;
 void *data;
};
# 193 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h" 2

struct kfilter_mapping {
 char *name;
 size_t len;
 uint32_t filter;
};
# 228 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
struct filterops {
 int f_flags;
 int (*f_attach) (struct knote *);

 void (*f_detach) (struct knote *);

 int (*f_event) (struct knote *, long);

 int (*f_touch) (struct knote *, struct kevent *, long);
};
# 250 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
struct kfilter;

struct knote {
 struct { struct knote *sle_next; } kn_link;
 struct { struct knote *sle_next; } kn_selnext;
 struct { struct knote *tqe_next; struct knote * *tqe_prev; } kn_tqe;
 struct kqueue *kn_kq;
 struct kevent kn_kevent;
 uint32_t kn_status;
 uint32_t kn_sfflags;
 uintptr_t kn_sdata;
 void *kn_obj;
 const struct filterops *kn_fop;
 struct kfilter *kn_kfilter;
 void *kn_hook;
 int kn_hookid;
# 297 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
};



struct lwp;
struct timespec;

void kqueue_init(void);
void knote(struct klist *, long);
void knote_fdclose(int);
void knote_set_eof(struct knote *, uint32_t);
void knote_clear_eof(struct knote *);

typedef int (*kevent_fetch_changes_t)(void *, const struct kevent *,
    struct kevent *, size_t, int);
typedef int (*kevent_put_events_t)(void *, struct kevent *, struct kevent *,
    size_t, int);

struct kevent_ops {
 void *keo_private;
 copyin_t keo_fetch_timeout;
 kevent_fetch_changes_t keo_fetch_changes;
 kevent_put_events_t keo_put_events;
};


int kevent_fetch_changes(void *, const struct kevent *, struct kevent *,
    size_t, int);
int kevent_put_events(void *, struct kevent *, struct kevent *, size_t,
    int);
int kevent1(register_t *, int, const struct kevent *,
    size_t, struct kevent *, size_t, const struct timespec *,
    const struct kevent_ops *);

int kfilter_register(const char *, const struct filterops *, int *);
int kfilter_unregister(const char *);

int filt_seltrue(struct knote *, long);
extern const struct filterops seltrue_filtops;

void klist_init(struct klist *);
void klist_fini(struct klist *);
void klist_insert(struct klist *, struct knote *);

# 340 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h" 3 4
_Bool 
# 340 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/event.h"
    klist_remove(struct klist *, struct knote *);
# 68 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/selinfo.h" 2





struct selinfo {
 uint64_t sel_collision;
 struct klist sel_klist;
 void *sel_cluster;
 struct lwp *sel_lwp;
 uintptr_t sel_fdinfo;
 struct { struct selinfo *sle_next; } sel_chain;
 uintptr_t sel_reserved[2];
};
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sigtypes.h" 1
# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sigtypes.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 49 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sigtypes.h" 2
# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sigtypes.h"
typedef struct {
 __uint32_t __bits[4];
} sigset_t;
# 109 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sigtypes.h"
typedef struct

               sigaltstack

      {
 void *ss_sp;
 size_t ss_size;
 int ss_flags;
} stack_t;
# 44 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h" 2
# 112 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/siginfo.h" 1
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/siginfo.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/signal.h" 1




typedef int sig_atomic_t;
# 36 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/siginfo.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/siginfo.h" 2




typedef union sigval {
 int sival_int;
 void *sival_ptr;
} sigval_t;

struct _ksiginfo {
 int _signo;
 int _code;
 int _errno;


 int _pad;

 union {
  struct {
   __pid_t _pid;
   __uid_t _uid;
   sigval_t _value;
  } _rt;

  struct {
   __pid_t _pid;
   __uid_t _uid;
   int _status;
   clock_t _utime;
   clock_t _stime;
  } _child;

  struct {
   void *_addr;
   int _trap;
   int _trap2;
   int _trap3;
  } _fault;

  struct {
   long _band;
   int _fd;
  } _poll;

  struct {
   int _sysnum;
   int _retval[2];
   int _error;
   uint64_t _args[8];
  } _syscall;

  struct {
   int _pe_report_event;
   union {
    __pid_t _pe_other_pid;
    lwpid_t _pe_lwp;
   } _option;
  } _ptrace_state;
 } _reason;
};


typedef struct ksiginfo {
 u_long ksi_flags;
 struct { struct ksiginfo *tqe_next; struct ksiginfo * *tqe_prev; } ksi_list;
 struct _ksiginfo ksi_info;
 lwpid_t ksi_lid;
} ksiginfo_t;
# 148 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/siginfo.h"
typedef union siginfo {
 char si_pad[128];
 struct _ksiginfo _info;
} siginfo_t;
# 113 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h" 2




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ucontext.h" 1
# 66 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ucontext.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/mcontext.h" 1




typedef struct mcontext {
 long __gregs[26];
} mcontext_t;
# 67 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ucontext.h" 2

typedef struct __ucontext ucontext_t;

struct __ucontext {
 unsigned int uc_flags;
 ucontext_t * uc_link;
 sigset_t uc_sigmask;
 stack_t uc_stack;
 mcontext_t uc_mcontext;



};
# 102 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ucontext.h"
struct lwp;

void getucontext(struct lwp *, ucontext_t *);
int setucontext(struct lwp *, const ucontext_t *);
void cpu_getmcontext(struct lwp *, mcontext_t *, unsigned int *);
int cpu_setmcontext(struct lwp *, const mcontext_t *, unsigned int);
int cpu_mcontext_validate(struct lwp *, const mcontext_t *);


struct __ctassert0_struct { unsigned int __ctassert0 : (sizeof(ucontext_t) == 784) ? 1 : -1; };
# 118 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h" 2





struct sigaction {
 union {
  void (*_sa_handler)(int);


  void (*_sa_sigaction)(int, siginfo_t *, void *);

 } _sa_u;
 sigset_t sa_mask;
 int sa_flags;
};
# 247 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
typedef void (*sig_t)(int);
# 277 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
struct sigstack {
 void *ss_sp;
 int ss_onstack;
};
# 297 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
struct sigevent {
 int sigev_notify;
 int sigev_signo;
 union sigval sigev_value;
 void (*sigev_notify_function)(union sigval);
 void *sigev_notify_attributes;
};
# 319 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"

# 319 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
#pragma GCC visibility push(default)
# 319 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"

void (*signal(int, void (*)(int)))(int);

int sigqueue(__pid_t, int, const union sigval);
# 336 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
void (*bsd_signal(int, void (*)(int)))(int);


int sigqueueinfo(__pid_t, const siginfo_t *);


# 341 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"
#pragma GCC visibility pop
# 341 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signal.h"

# 44 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h" 2

struct lwp;
struct proc;
struct timespec;
struct cpu_info;
struct socket;
struct knote;

int selcommon(register_t *, int, fd_set *, fd_set *, fd_set *,
    struct timespec *, sigset_t *);
void selrecord(struct lwp *selector, struct selinfo *);
void selrecord_knote(struct selinfo *, struct knote *);

# 56 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h" 3 4
_Bool 
# 56 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/select.h"
    selremove_knote(struct selinfo *, struct knote *);
void selnotify(struct selinfo *, int, long);
void selsysinit(struct cpu_info *);
void selinit(struct selinfo *);
void seldestroy(struct selinfo *);
# 67 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 2


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h" 1
# 140 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h"
typedef enum kmutex_type_t {
 MUTEX_SPIN = 0,
 MUTEX_ADAPTIVE = 1,
 MUTEX_DEFAULT = 2,
 MUTEX_DRIVER = 3,
 MUTEX_NODEBUG = 4
} kmutex_type_t;

typedef struct kmutex kmutex_t;
# 174 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/intr.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/intr.h"
struct cpu_info;


void *softint_establish(u_int, void (*)(void *), void *);
void softint_disestablish(void *);
void softint_schedule(void *);
void softint_schedule_cpu(void *, struct cpu_info *);


void softint_init(struct cpu_info *);
lwp_t *softint_picklwp(void);
void softint_block(lwp_t *);


void softint_init_md(lwp_t *, u_int, uintptr_t *);

void softint_trigger(uintptr_t);

void softint_dispatch(lwp_t *, int);
# 77 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/intr.h"
extern u_int softint_timing;
# 96 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/intr.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/intr.h" 1
# 97 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/intr.h" 2
# 175 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h" 2


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/mutex.h" 1




struct kmutex {
 uintptr_t mtx_owner;
 uint8_t mtx_ipl;
 uint8_t mtx_lock;
};
# 178 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h" 2
# 188 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h"
void _mutex_init(kmutex_t *, kmutex_type_t, int, uintptr_t);
void mutex_init(kmutex_t *, kmutex_type_t, int);
void mutex_destroy(kmutex_t *);

void mutex_enter(kmutex_t *);
void mutex_exit(kmutex_t *);

void mutex_spin_enter(kmutex_t *);
void mutex_spin_exit(kmutex_t *);

int mutex_tryenter(kmutex_t *);

int mutex_owned(const kmutex_t *);
int mutex_ownable(const kmutex_t *);

void mutex_obj_init(void);
kmutex_t *mutex_obj_alloc(kmutex_type_t, int);
void mutex_obj_hold(kmutex_t *);

# 206 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h" 3 4
_Bool 
# 206 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/mutex.h"
    mutex_obj_free(kmutex_t *);
u_int mutex_obj_refcnt(kmutex_t *);
kmutex_t *mutex_obj_tryalloc(kmutex_type_t, int);
# 70 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/condvar.h" 1
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/condvar.h"
typedef struct kcondvar {
 void *cv_opaque[2];
} kcondvar_t;



struct bintime;
struct kmutex;
struct timespec;

void cv_init(kcondvar_t *, const char *);
void cv_destroy(kcondvar_t *);

void cv_wait(kcondvar_t *, struct kmutex *);
int cv_wait_sig(kcondvar_t *, struct kmutex *);
int cv_timedwait(kcondvar_t *, struct kmutex *, int);
int cv_timedwait_sig(kcondvar_t *, struct kmutex *, int);
int cv_timedwaitbt(kcondvar_t *, struct kmutex *, struct bintime *,
     const struct bintime *);
int cv_timedwaitbt_sig(kcondvar_t *, struct kmutex *, struct bintime *,
     const struct bintime *);

void cv_signal(kcondvar_t *);
void cv_broadcast(kcondvar_t *);


# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/condvar.h" 3 4
_Bool 
# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/condvar.h"
    cv_has_waiters(kcondvar_t *);

# 61 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/condvar.h" 3 4
_Bool 
# 61 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/condvar.h"
    cv_is_valid(kcondvar_t *);


extern kcondvar_t lbolt;
# 71 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 2






# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h" 1
# 133 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h"

# 133 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h"
#pragma GCC visibility push(default)
# 133 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h"


void atomic_add_32(volatile uint32_t *, int32_t); uint32_t atomic_add_32_nv(volatile uint32_t *, int32_t);
void atomic_add_64(volatile uint64_t *, int64_t); uint64_t atomic_add_64_nv(volatile uint64_t *, int64_t);
void atomic_add_int(volatile unsigned int *, int); unsigned int atomic_add_int_nv(volatile unsigned int *, int);
void atomic_add_long(volatile unsigned long *, long); unsigned long atomic_add_long_nv(volatile unsigned long *, long);
void atomic_add_ptr(volatile void *, ssize_t); void * atomic_add_ptr_nv(volatile void *, ssize_t);

void atomic_and_32(volatile uint32_t *, uint32_t); uint32_t atomic_and_32_nv(volatile uint32_t *, uint32_t);
void atomic_and_64(volatile uint64_t *, uint64_t); uint64_t atomic_and_64_nv(volatile uint64_t *, uint64_t);
void atomic_and_uint(volatile unsigned int *, unsigned int); unsigned int atomic_and_uint_nv(volatile unsigned int *, unsigned int);
void atomic_and_ulong(volatile unsigned long *, unsigned long); unsigned long atomic_and_ulong_nv(volatile unsigned long *, unsigned long);

void atomic_or_32(volatile uint32_t *, uint32_t); uint32_t atomic_or_32_nv(volatile uint32_t *, uint32_t);
void atomic_or_64(volatile uint64_t *, uint64_t); uint64_t atomic_or_64_nv(volatile uint64_t *, uint64_t);
void atomic_or_uint(volatile unsigned int *, unsigned int); unsigned int atomic_or_uint_nv(volatile unsigned int *, unsigned int);
void atomic_or_ulong(volatile unsigned long *, unsigned long); unsigned long atomic_or_ulong_nv(volatile unsigned long *, unsigned long);

uint32_t atomic_cas_32(volatile uint32_t *, uint32_t, uint32_t); uint32_t atomic_cas_32_ni(volatile uint32_t *, uint32_t, uint32_t);
uint64_t atomic_cas_64(volatile uint64_t *, uint64_t, uint64_t); uint64_t atomic_cas_64_ni(volatile uint64_t *, uint64_t, uint64_t);
unsigned int atomic_cas_uint(volatile unsigned int *, unsigned int, unsigned int); unsigned int atomic_cas_uint_ni(volatile unsigned int *, unsigned int, unsigned int);
unsigned long atomic_cas_ulong(volatile unsigned long *, unsigned long, unsigned long); unsigned long atomic_cas_ulong_ni(volatile unsigned long *, unsigned long, unsigned long);
void * atomic_cas_ptr(volatile void *, void *, void *); void * atomic_cas_ptr_ni(volatile void *, void *, void *);

uint32_t atomic_swap_32(volatile uint32_t *, uint32_t);
uint64_t atomic_swap_64(volatile uint64_t *, uint64_t);
unsigned int atomic_swap_uint(volatile unsigned int *, unsigned int);
unsigned long atomic_swap_ulong(volatile unsigned long *, unsigned long);
void * atomic_swap_ptr(volatile void *, void *);

void atomic_dec_32(volatile uint32_t *); uint32_t atomic_dec_32_nv(volatile uint32_t *);
void atomic_dec_64(volatile uint64_t *); uint64_t atomic_dec_64_nv(volatile uint64_t *);
void atomic_dec_uint(volatile unsigned int *); unsigned int atomic_dec_uint_nv(volatile unsigned int *);
void atomic_dec_ulong(volatile unsigned long *); unsigned long atomic_dec_ulong_nv(volatile unsigned long *);
void atomic_dec_ptr(volatile void *); void * atomic_dec_ptr_nv(volatile void *);

void atomic_inc_32(volatile uint32_t *); uint32_t atomic_inc_32_nv(volatile uint32_t *);
void atomic_inc_64(volatile uint64_t *); uint64_t atomic_inc_64_nv(volatile uint64_t *);
void atomic_inc_uint(volatile unsigned int *); unsigned int atomic_inc_uint_nv(volatile unsigned int *);
void atomic_inc_ulong(volatile unsigned long *); unsigned long atomic_inc_ulong_nv(volatile unsigned long *);
void atomic_inc_ptr(volatile void *); void * atomic_inc_ptr_nv(volatile void *);





uint16_t atomic_cas_16(volatile uint16_t *, uint16_t, uint16_t);
uint8_t atomic_cas_8(volatile uint8_t *, uint8_t, uint8_t);




void membar_acquire(void);
void membar_release(void);
void membar_producer(void);
void membar_consumer(void);
void membar_sync(void);




void membar_enter(void);
void membar_exit(void);








# 203 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h"
#pragma GCC visibility pop
# 203 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h"

# 401 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h" 1
# 46 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/stddef.h" 1




# 1 "/usr/lib/gcc/x86_64-linux-gnu/14/include/stddef.h" 1 3 4
# 6 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/stddef.h" 2
# 47 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/container_of.h" 1
# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/strlist.h" 1
# 40 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/strlist.h"
const char * strlist_next(const char *, size_t, size_t *);
unsigned int strlist_count(const char *, size_t);
const char * strlist_string(const char *, size_t, unsigned int);

int strlist_match(const char *, size_t, const char *);
int strlist_pmatch(const char *, size_t, const char *);
int strlist_index(const char *, size_t, const char *);


# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/strlist.h" 3 4
_Bool 
# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/strlist.h"
     strlist_append(char **, size_t *, const char *);
# 50 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h" 2






static __inline int imax(int, int) __attribute__((__unused__));
static __inline int imin(int, int) __attribute__((__unused__));
static __inline u_int uimax(u_int, u_int) __attribute__((__unused__));
static __inline u_int uimin(u_int, u_int) __attribute__((__unused__));
static __inline long lmax(long, long) __attribute__((__unused__));
static __inline long lmin(long, long) __attribute__((__unused__));
static __inline u_long ulmax(u_long, u_long) __attribute__((__unused__));
static __inline u_long ulmin(u_long, u_long) __attribute__((__unused__));
static __inline int abs(int) __attribute__((__unused__));
static __inline long labs(long) __attribute__((__unused__));
static __inline long long llabs(long long) __attribute__((__unused__));
static __inline intmax_t imaxabs(intmax_t) __attribute__((__unused__));

static __inline int isspace(int) __attribute__((__unused__));
static __inline int isascii(int) __attribute__((__unused__));
static __inline int isupper(int) __attribute__((__unused__));
static __inline int islower(int) __attribute__((__unused__));
static __inline int isalpha(int) __attribute__((__unused__));
static __inline int isalnum(int) __attribute__((__unused__));
static __inline int isdigit(int) __attribute__((__unused__));
static __inline int isxdigit(int) __attribute__((__unused__));
static __inline int iscntrl(int) __attribute__((__unused__));
static __inline int isgraph(int) __attribute__((__unused__));
static __inline int isprint(int) __attribute__((__unused__));
static __inline int ispunct(int) __attribute__((__unused__));
static __inline int toupper(int) __attribute__((__unused__));
static __inline int tolower(int) __attribute__((__unused__));


static __inline int
imax(int a, int b)
{
 return (a > b ? a : b);
}
static __inline int
imin(int a, int b)
{
 return (a < b ? a : b);
}
static __inline long
lmax(long a, long b)
{
 return (a > b ? a : b);
}
static __inline long
lmin(long a, long b)
{
 return (a < b ? a : b);
}
static __inline u_int
uimax(u_int a, u_int b)
{
 return (a > b ? a : b);
}
static __inline u_int
uimin(u_int a, u_int b)
{
 return (a < b ? a : b);
}
static __inline u_long
ulmax(u_long a, u_long b)
{
 return (a > b ? a : b);
}
static __inline u_long
ulmin(u_long a, u_long b)
{
 return (a < b ? a : b);
}

static __inline int
abs(int j)
{
 return(j < 0 ? -j : j);
}

static __inline long
labs(long j)
{
 return(j < 0 ? -j : j);
}

static __inline long long
llabs(long long j)
{
 return(j < 0 ? -j : j);
}

static __inline intmax_t
imaxabs(intmax_t j)
{
 return(j < 0 ? -j : j);
}

static __inline int
isspace(int ch)
{
 return (ch == ' ' || (ch >= '\t' && ch <= '\r'));
}

static __inline int
isascii(int ch)
{
 return ((ch & ~0x7f) == 0);
}

static __inline int
isupper(int ch)
{
 return (ch >= 'A' && ch <= 'Z');
}

static __inline int
islower(int ch)
{
 return (ch >= 'a' && ch <= 'z');
}

static __inline int
isalpha(int ch)
{
 return (isupper(ch) || islower(ch));
}

static __inline int
isalnum(int ch)
{
 return (isalpha(ch) || isdigit(ch));
}

static __inline int
isdigit(int ch)
{
 return (ch >= '0' && ch <= '9');
}

static __inline int
isxdigit(int ch)
{
 return (isdigit(ch) ||
     (ch >= 'A' && ch <= 'F') ||
     (ch >= 'a' && ch <= 'f'));
}

static __inline int
iscntrl(int ch)
{
 return ((ch >= 0x00 && ch <= 0x1F) || ch == 0x7F);
}

static __inline int
isgraph(int ch)
{
 return (ch != ' ' && isprint(ch));
}

static __inline int
isprint(int ch)
{
 return (ch >= 0x20 && ch <= 0x7E);
}

static __inline int
ispunct(int ch)
{
 return (isprint(ch) && ch != ' ' && !isalnum(ch));
}

static __inline int
toupper(int ch)
{
 if (islower(ch))
  return (ch - 0x20);
 return (ch);
}

static __inline int
tolower(int ch)
{
 if (isupper(ch))
  return (ch + 0x20);
 return (ch);
}
# 320 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
void *memcpy(void *, const void *, size_t);
int memcmp(const void *, const void *, size_t);
void *memset(void *, int, size_t);
# 351 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
void *memmem(const void *, size_t, const void *, size_t);

char *strcpy(char *, const char *);
int strcmp(const char *, const char *);
size_t strlen(const char *);
# 384 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
size_t strnlen(const char *, size_t);
char *strsep(char **, const char *);







char *strcat(char *, const char *);
char *strchr(const char *, int);
char *strrchr(const char *, int);
# 411 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
size_t strcspn(const char *, const char *);
char *strncpy(char *, const char *, size_t);
char *strncat(char *, const char *, size_t);
int strncmp(const char *, const char *, size_t);
char *strstr(const char *, const char *);
char *strpbrk(const char *, const char *);
size_t strspn(const char *, const char *);




int ffs(int);




void kern_assert(const char *, ...)
    __attribute__((__format__(__printf__, 1, 2)));
u_int32_t
 inet_addr(const char *);
struct in_addr;
int inet_aton(const char *, struct in_addr *);
char *intoa(u_int32_t);

void *memchr(const void *, int, size_t);

void *memmove(void *, const void *, size_t);
# 449 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
int pmatch(const char *, const char *, const char **);





long random(void);
void mi_vector_hash(const void * restrict, size_t, uint32_t,
     uint32_t[3]);
int scanc(u_int, const u_char *, const u_char *, int);
int skpc(int, size_t, u_char *);
int strcasecmp(const char *, const char *);
size_t strlcpy(char * restrict, const char * restrict, size_t);
size_t strlcat(char * restrict, const char * restrict, size_t);
int strncasecmp(const char *, const char *, size_t);
u_long strtoul(const char *, char **, int);
long long strtoll(const char *, char **, int);
unsigned long long strtoull(const char *, char **, int);
intmax_t strtoimax(const char *, char **, int);
uintmax_t strtoumax(const char *, char **, int);
intmax_t strtoi(const char * restrict, char ** restrict, int, intmax_t,
    intmax_t, int *);
uintmax_t strtou(const char * restrict, char ** restrict, int, uintmax_t,
    uintmax_t, int *);
void hexdump(void (*)(const char *, ...) __attribute__((__format__ (__printf__, 1, 2))),
    const char *, const void *, size_t);

int snprintb(char *, size_t, const char *, uint64_t);
int snprintb_m(char *, size_t, const char *, uint64_t, size_t);
int kheapsort(void *, size_t, size_t, int (*)(const void *, const void *),
     void *);
int kheapsort_r(void *, size_t, size_t,
     int (*)(const void *, const void *, void *), void *,
     void *);
uint32_t crc32(uint32_t, const uint8_t *, size_t);
# 492 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/lib/libkern/libkern.h"
unsigned int popcount(unsigned int) __attribute__((__const__));
unsigned int popcountl(unsigned long) __attribute__((__const__));
unsigned int popcountll(unsigned long long) __attribute__((__const__));
unsigned int popcount32(uint32_t) __attribute__((__const__));
unsigned int popcount64(uint64_t) __attribute__((__const__));


void *explicit_memset(void *, int, size_t);
int consttime_memequal(const void *, const void *, size_t);
int strnvisx(char *, size_t, const char *, size_t, int);




struct disklabel;
void disklabel_swap(struct disklabel *, struct disklabel *);
uint16_t dkcksum(const struct disklabel *);
uint16_t dkcksum_sized(const struct disklabel *, size_t);
# 402 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/atomic.h" 2
# 78 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/uidinfo.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/uidinfo.h"
struct uidinfo {
 struct { struct uidinfo *sle_next; } ui_hash;
 __uid_t ui_uid;
 u_long ui_proccnt;
 u_long ui_lwpcnt;
 u_long ui_lockcnt;
 u_long ui_semcnt;
 u_long ui_sbsize;
};

int chgproccnt(__uid_t, int);
int chglwpcnt(__uid_t, int);
int chgsemcnt(__uid_t, int);
int chgsbsize(struct uidinfo *, u_long *, u_long, rlim_t);
struct uidinfo *uid_find(__uid_t);
void uid_init(void);
# 79 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 2


struct soqhead { struct socket *tqh_first; struct socket * *tqh_last; };




struct sockbuf {
 struct selinfo sb_sel;
 struct mowner *sb_mowner;
 struct socket *sb_so;
 kcondvar_t sb_cv;


 u_long sb_cc;
 u_long sb_hiwat;
 u_long sb_mbcnt;
 u_long sb_mbmax;
 u_long sb_lowat;
 struct mbuf *sb_mb;
 struct mbuf *sb_mbtail;
 struct mbuf *sb_lastrecord;

 int sb_flags;
 int sb_timeo;
 u_long sb_overflowed;
};
# 125 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
struct so_accf {
 struct accept_filter *so_accept_filter;
 void *so_accept_filter_arg;
 char *so_accept_filter_str;
};

struct sockaddr;

struct socket {
 kmutex_t * volatile so_lock;
 kcondvar_t so_cv;
 short so_type;
 short so_options;
 u_short so_linger;
 short so_state;
 int so_unused;
 void *so_pcb;
 const struct protosw *so_proto;
# 154 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
 struct socket *so_head;
 struct soqhead *so_onq;
 struct soqhead so_q0;
 struct soqhead so_q;
 struct { struct socket *tqe_next; struct socket * *tqe_prev; } so_qe;
 short so_q0len;
 short so_qlen;
 short so_qlimit;
 short so_timeo;
 u_short so_error;
 u_short so_rerror;
 u_short so_aborting;
 __pid_t so_pgid;
 u_long so_oobmark;
 struct sockbuf so_snd;
 struct sockbuf so_rcv;

 void *so_internal;
 void (*so_upcall) (struct socket *, void *, int, int);
 void * so_upcallarg;
 int (*so_send) (struct socket *, struct sockaddr *,
     struct uio *, struct mbuf *,
     struct mbuf *, int, struct lwp *);
 int (*so_receive) (struct socket *,
     struct mbuf **,
     struct uio *, struct mbuf **,
     struct mbuf **, int *);
 struct mowner *so_mowner;
 struct uidinfo *so_uidinfo;
 __gid_t so_egid;
 __pid_t so_cpid;
 struct so_accf *so_accf;
 kauth_cred_t so_cred;
};
# 212 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
struct accept_filter {
 char accf_name[16];
 void (*accf_callback)
  (struct socket *, void *, int, int);
 void * (*accf_create)
  (struct socket *, char *);
 void (*accf_destroy)
  (struct socket *);
 struct { struct accept_filter *le_next; struct accept_filter **le_prev; } accf_next;
 u_int accf_refcnt;
};

struct sockopt {
 int sopt_level;
 int sopt_name;
 size_t sopt_size;
 size_t sopt_retsize;
 void * sopt_data;
 uint8_t sopt_buf[sizeof(int)];
};
# 242 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
extern u_long sb_max;
extern int somaxkva;
extern int sock_loan_thresh;
extern kmutex_t *softnet_lock;

struct mbuf;
struct lwp;
struct msghdr;
struct stat;
struct knote;
struct sockaddr_big;
enum uio_seg;







int soo_read(file_t *, __off_t *, struct uio *, kauth_cred_t, int);
int soo_write(file_t *, __off_t *, struct uio *, kauth_cred_t, int);
int soo_fcntl(file_t *, u_int cmd, void *);
int soo_ioctl(file_t *, u_long cmd, void *);
int soo_poll(file_t *, int);
int soo_kqfilter(file_t *, struct knote *);
int soo_close(file_t *);
int soo_stat(file_t *, struct stat *);
void soo_restart(file_t *);
void sbappend(struct sockbuf *, struct mbuf *);
void sbappendstream(struct sockbuf *, struct mbuf *);
int sbappendaddr(struct sockbuf *, const struct sockaddr *, struct mbuf *,
     struct mbuf *);
int sbappendaddrchain(struct sockbuf *, const struct sockaddr *,
      struct mbuf *, int);
int sbappendcontrol(struct sockbuf *, struct mbuf *, struct mbuf *);
void sbappendrecord(struct sockbuf *, struct mbuf *);
void sbcheck(struct sockbuf *);
void sbcompress(struct sockbuf *, struct mbuf *, struct mbuf *);
struct mbuf *
 sbcreatecontrol(void *, int, int, int);
struct mbuf *
 sbcreatecontrol1(void **, int, int, int, int);
struct mbuf **
 sbsavetimestamp(int, struct mbuf **);
void sbdrop(struct sockbuf *, int);
void sbdroprecord(struct sockbuf *);
void sbflush(struct sockbuf *);
void sbinsertoob(struct sockbuf *, struct mbuf *);
void sbrelease(struct sockbuf *, struct socket *);
int sbreserve(struct sockbuf *, u_long, struct socket *);
int sbwait(struct sockbuf *);
int sb_max_set(u_long);
void soinit(void);
void soinit1(void);
void soinit2(void);
int soabort(struct socket *);
int soaccept(struct socket *, struct sockaddr *);
int sofamily(const struct socket *);
int sobind(struct socket *, struct sockaddr *, struct lwp *);
void socantrcvmore(struct socket *);
void socantsendmore(struct socket *);
void soroverflow(struct socket *);
int soclose(struct socket *);
int soconnect(struct socket *, struct sockaddr *, struct lwp *);
int soconnect2(struct socket *, struct socket *);
int socreate(int, struct socket **, int, int, struct lwp *,
   struct socket *);
int fsocreate(int, struct socket **, int, int, int *, file_t **,
  struct socket *);
int sodisconnect(struct socket *);
void sofree(struct socket *);
int sogetopt(struct socket *, struct sockopt *);
void sohasoutofband(struct socket *);
void soisconnected(struct socket *);
void soisconnecting(struct socket *);
void soisdisconnected(struct socket *);
void soisdisconnecting(struct socket *);
int solisten(struct socket *, int, struct lwp *);
struct socket *
 sonewconn(struct socket *, 
# 321 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
                           _Bool
# 321 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
                               );
void soqinsque(struct socket *, struct socket *, int);

# 323 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
_Bool 
# 323 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
    soqremque(struct socket *, int);
int soreceive(struct socket *, struct mbuf **, struct uio *,
     struct mbuf **, struct mbuf **, int *);
int soreserve(struct socket *, u_long, u_long);
void sorflush(struct socket *);
int sosend(struct socket *, struct sockaddr *, struct uio *,
     struct mbuf *, struct mbuf *, int, struct lwp *);
int sosetopt(struct socket *, struct sockopt *);
int so_setsockopt(struct lwp *, struct socket *, int, int, const void *, size_t);
int soshutdown(struct socket *, int);
void sorestart(struct socket *);
void sowakeup(struct socket *, struct sockbuf *, int);
int sockargs(struct mbuf **, const void *, size_t, enum uio_seg, int);
int sopoll(struct socket *, int);
struct socket *soget(
# 337 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
                    _Bool
# 337 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
                        );
void soput(struct socket *);

# 339 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
_Bool 
# 339 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
    solocked(const struct socket *);

# 340 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
_Bool 
# 340 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
    solocked2(const struct socket *, const struct socket *);
int sblock(struct sockbuf *, int);
void sbunlock(struct sockbuf *);
int sowait(struct socket *, 
# 343 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
                           _Bool
# 343 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
                               , int);
void solockretry(struct socket *, kmutex_t *);
void sosetlock(struct socket *);
void solockreset(struct socket *, kmutex_t *);

void sockopt_init(struct sockopt *, int, int, size_t);
void sockopt_destroy(struct sockopt *);
int sockopt_set(struct sockopt *, const void *, size_t);
int sockopt_setint(struct sockopt *, int);
int sockopt_get(const struct sockopt *, void *, size_t);
int sockopt_getint(const struct sockopt *, int *);
int sockopt_setmbuf(struct sockopt *, struct mbuf *);
struct mbuf *sockopt_getmbuf(const struct sockopt *);

int copyout_sockname(struct sockaddr *, unsigned int *, int, struct mbuf *);
int copyout_sockname_sb(struct sockaddr *, unsigned int *,
    int , struct sockaddr_big *);
int copyout_msg_control(struct lwp *, struct msghdr *, struct mbuf *);
void free_control_mbuf(struct lwp *, struct mbuf *, struct mbuf *);

int do_sys_getpeername(int, struct sockaddr *);
int do_sys_getsockname(int, struct sockaddr *);

int do_sys_sendmsg(struct lwp *, int, struct msghdr *, int, register_t *);
int do_sys_sendmsg_so(struct lwp *, int, struct socket *, file_t *,
     struct msghdr *, int, register_t *);

int do_sys_recvmsg(struct lwp *, int, struct msghdr *,
     struct mbuf **, struct mbuf **, register_t *);
int do_sys_recvmsg_so(struct lwp *, int, struct socket *,
     struct msghdr *mp, struct mbuf **, struct mbuf **, register_t *);

int do_sys_bind(struct lwp *, int, struct sockaddr *);
int do_sys_connect(struct lwp *, int, struct sockaddr *);
int do_sys_accept(struct lwp *, int, struct sockaddr *, register_t *,
     const sigset_t *, int, int);

int do_sys_peeloff(struct socket *, void *);




# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/protosw.h" 1
# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/protosw.h"
struct mbuf;
struct ifnet;
struct sockaddr;
struct socket;
struct sockopt;
struct stat;
struct domain;
struct proc;
struct lwp;
struct pr_usrreqs;

struct protosw {
 int pr_type;
 struct domain *pr_domain;
 short pr_protocol;
 short pr_flags;


 void (*pr_input)
   (struct mbuf *, int, int);
 void *(*pr_ctlinput)
   (int, const struct sockaddr *, void *);
 int (*pr_ctloutput)
   (int, struct socket *, struct sockopt *);


 const struct pr_usrreqs *pr_usrreqs;


 void (*pr_init)
   (void);

 void (*pr_fasttimo)
   (void);
 void (*pr_slowtimo)
   (void);
 void (*pr_drain)
   (void);
};
# 238 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/protosw.h"
struct pr_usrreqs {
 int (*pr_attach)(struct socket *, int);
 void (*pr_detach)(struct socket *);
 int (*pr_accept)(struct socket *, struct sockaddr *);
 int (*pr_connect)(struct socket *, struct sockaddr *, struct lwp *);
 int (*pr_connect2)(struct socket *, struct socket *);
 int (*pr_bind)(struct socket *, struct sockaddr *, struct lwp *);
 int (*pr_listen)(struct socket *, struct lwp *);
 int (*pr_disconnect)(struct socket *);
 int (*pr_shutdown)(struct socket *);
 int (*pr_abort)(struct socket *);
 int (*pr_ioctl)(struct socket *, u_long, void *, struct ifnet *);
 int (*pr_stat)(struct socket *, struct stat *);
 int (*pr_peeraddr)(struct socket *, struct sockaddr *);
 int (*pr_sockaddr)(struct socket *, struct sockaddr *);
 int (*pr_rcvd)(struct socket *, int, struct lwp *);
 int (*pr_recvoob)(struct socket *, struct mbuf *, int);
 int (*pr_send)(struct socket *, struct mbuf *, struct sockaddr *,
     struct mbuf *, struct lwp *);
 int (*pr_sendoob)(struct socket *, struct mbuf *, struct mbuf *);
 int (*pr_purgeif)(struct socket *, struct ifnet *);
};




extern u_int pfslowtimo_now;
extern u_int pffasttimo_now;
# 279 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/protosw.h"
struct sockaddr;
const struct protosw *pffindproto(int, int, int);
const struct protosw *pffindtype(int, int);
struct domain *pffinddomain(int);
void pfctlinput(int, const struct sockaddr *);
void pfctlinput2(int, const struct sockaddr *, void *);
# 497 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/protosw.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 1
# 498 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/protosw.h" 2
# 386 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 2





static __inline int
sb_notify(struct sockbuf *sb)
{

 ((void)sizeof((long)(solocked(sb->sb_so))));

 return sb->sb_flags & (0x04 | 0x10 | 0x20 | 0x100);
}





static __inline u_long
sbspace(const struct sockbuf *sb)
{

 ((void)sizeof((long)(solocked(sb->sb_so))));
 if (sb->sb_hiwat <= sb->sb_cc || sb->sb_mbmax <= sb->sb_mbcnt)
  return 0;
 return lmin(sb->sb_hiwat - sb->sb_cc, sb->sb_mbmax - sb->sb_mbcnt);
}

static __inline u_long
sbspace_oob(const struct sockbuf *sb)
{
 u_long hiwat = sb->sb_hiwat;

 if (hiwat < 
# 419 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h" 3 4
            (0x7fffffffffffffffL * 2UL + 1UL) 
# 419 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
                      - 1024)
  hiwat += 1024;

 ((void)sizeof((long)(solocked(sb->sb_so))));

 if (hiwat <= sb->sb_cc || sb->sb_mbmax <= sb->sb_mbcnt)
  return 0;
 return lmin(hiwat - sb->sb_cc, sb->sb_mbmax - sb->sb_mbcnt);
}




static __inline u_long
sbused(const struct sockbuf *sb)
{

 ((void)sizeof((long)(solocked(sb->sb_so))));
 return sb->sb_cc;
}


static __inline int
sosendallatonce(const struct socket *so)
{

 return so->so_proto->pr_flags & 0x01;
}


static __inline int
soreadable(const struct socket *so)
{

 ((void)sizeof((long)(solocked(so))));

 return so->so_rcv.sb_cc >= so->so_rcv.sb_lowat ||
     (so->so_state & 0x020) != 0 ||
     so->so_qlen != 0 || so->so_error != 0 || so->so_rerror != 0;
}


static __inline int
sowritable(const struct socket *so)
{

 ((void)sizeof((long)(solocked(so))));

 return (sbspace(&so->so_snd) >= so->so_snd.sb_lowat &&
     ((so->so_state & 0x002) != 0 ||
     (so->so_proto->pr_flags & 0x04) == 0)) ||
     (so->so_state & 0x010) != 0 ||
     so->so_error != 0;
}


static __inline void
sballoc(struct sockbuf *sb, struct mbuf *m)
{

 ((void)sizeof((long)(solocked(sb->sb_so))));

 sb->sb_cc += m->m_len;
 sb->sb_mbcnt += 256;
 if (m->m_flags & 0x0001)
  sb->sb_mbcnt += m->m_ext.ext_size;
}


static __inline void
sbfree(struct sockbuf *sb, struct mbuf *m)
{

 ((void)sizeof((long)(solocked(sb->sb_so))));

 sb->sb_cc -= m->m_len;
 sb->sb_mbcnt -= 256;
 if (m->m_flags & 0x0001)
  sb->sb_mbcnt -= m->m_ext.ext_size;
}

static __inline void
sorwakeup(struct socket *so)
{

 ((void)sizeof((long)(solocked(so))));

 if (sb_notify(&so->so_rcv))
  sowakeup(so, &so->so_rcv, 1);
}

static __inline void
sowwakeup(struct socket *so)
{

 ((void)sizeof((long)(solocked(so))));

 if (sb_notify(&so->so_snd))
  sowakeup(so, &so->so_snd, 2);
}

static __inline void
solock(struct socket *so)
{
 kmutex_t *lock;

 lock = ({ const volatile __typeof__(*(&so->so_lock)) *__al_ptr = (&so->so_lock); do { struct __ctassert1_struct { unsigned int __ctassert1 : (sizeof(*(__al_ptr)) <= 8) ? 1 : -1; }; ((void)sizeof((long)(((uintptr_t)(__al_ptr) & (sizeof(*(__al_ptr)) - 1)) == 0))); } while (0); const __typeof_unqual__(*(__al_ptr)) __al_val = *(__al_ptr); ((void)0); __al_val; });
 mutex_enter(lock);
 if (__builtin_expect((lock != ({ const volatile __typeof__(*(&so->so_lock)) *__al_ptr = (&so->so_lock); do { struct __ctassert2_struct { unsigned int __ctassert2 : (sizeof(*(__al_ptr)) <= 8) ? 1 : -1; }; ((void)sizeof((long)(((uintptr_t)(__al_ptr) & (sizeof(*(__al_ptr)) - 1)) == 0))); } while (0); const __typeof_unqual__(*(__al_ptr)) __al_val = *(__al_ptr); __al_val; })) ? 1 : 0, 0))
  solockretry(so, lock);
}

static __inline void
sounlock(struct socket *so)
{

 mutex_exit(so->so_lock);
}
# 559 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
vaddr_t sokvaalloc(vaddr_t, vsize_t, struct socket *);
void sokvafree(vaddr_t, vsize_t);
void soloanfree(struct mbuf *, void *, size_t, void *);
# 592 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/socketvar.h"
int accept_filt_getopt(struct socket *, struct sockopt *);
int accept_filt_setopt(struct socket *, const struct sockopt *);
int accept_filt_clear(struct socket *);
int accept_filt_add(struct accept_filter *);
int accept_filt_del(struct accept_filter *);
struct accept_filter *accept_filt_get(char *);
# 111 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/domain.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/domain.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 1
# 40 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/net/if.h" 1






struct ifnet {
    void *if_softc;
    char if_xname[16];
    int if_flags;
    int if_mtu;
    int if_type;


    int (*if_init)(struct ifnet *);
    int (*if_ioctl)(struct ifnet *, u_long, void *);
    void (*if_start)(struct ifnet *);


    struct ifnet *if_next;
};
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/net/if.h"
extern struct ifnet *ifnet;

void if_attach(struct ifnet *ifp);
void if_detach(struct ifnet *ifp);
# 41 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/rwlock.h" 1
# 53 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/rwlock.h"
typedef enum krw_t {
 RW_READER = 0,
 RW_WRITER = 1
} krw_t;

typedef struct krwlock krwlock_t;
# 89 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/rwlock.h"
struct krwlock {
 volatile uintptr_t rw_owner;
};



void _rw_init(krwlock_t *, uintptr_t);
void rw_init(krwlock_t *);
void rw_destroy(krwlock_t *);

int rw_tryenter(krwlock_t *, const krw_t);
int rw_tryupgrade(krwlock_t *);
void rw_downgrade(krwlock_t *);

int rw_read_held(krwlock_t *);
int rw_write_held(krwlock_t *);
int rw_lock_held(krwlock_t *);
krw_t rw_lock_op(krwlock_t *);

void rw_enter(krwlock_t *, const krw_t);
void rw_exit(krwlock_t *);

void rw_obj_init(void);
krwlock_t *rw_obj_alloc(void);
void rw_obj_hold(krwlock_t *);

# 114 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/rwlock.h" 3 4
_Bool 
# 114 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/rwlock.h"
    rw_obj_free(krwlock_t *);
u_int rw_obj_refcnt(krwlock_t *);
krwlock_t *rw_obj_tryalloc(void);
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/pserialize.h" 1
# 34 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/pserialize.h"
struct pserialize;
typedef struct pserialize *pserialize_t;

void pserialize_init(void);

pserialize_t pserialize_create(void);
void pserialize_destroy(pserialize_t);
void pserialize_perform(pserialize_t);

int pserialize_read_enter(void);
void pserialize_read_exit(int);


# 46 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/pserialize.h" 3 4
_Bool 
# 46 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/pserialize.h"
     pserialize_in_read_section(void);

# 47 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/pserialize.h" 3 4
_Bool 
# 47 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/pserialize.h"
     pserialize_not_in_read_section(void);
# 45 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/percpu.h" 1
# 32 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/percpu.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/percpu_types.h" 1
# 34 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/percpu_types.h"
struct cpu_info;
typedef struct percpu percpu_t;

typedef struct percpu_cpu {
 size_t pcc_size;
 void *pcc_data;
} percpu_cpu_t;
# 33 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/percpu.h" 2

void percpu_init(void);
void percpu_init_cpu(struct cpu_info *);
percpu_t *percpu_alloc(size_t);
void percpu_free(percpu_t *, size_t);
void *percpu_getref(percpu_t *);
void percpu_putref(percpu_t *);

typedef void (*percpu_callback_t)(void *, void *, struct cpu_info *);
void percpu_foreach(percpu_t *, percpu_callback_t, void *);
void percpu_foreach_xcall(percpu_t *, u_int, percpu_callback_t, void *);

percpu_t *percpu_create(size_t, percpu_callback_t, percpu_callback_t, void *);


void percpu_traverse_enter(void);
void percpu_traverse_exit(void);
void *percpu_getptr_remote(percpu_t *, struct cpu_info *);
# 46 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h"
struct cpu_info;
struct lwp;

struct psref;
struct psref_class;
struct psref_target;
# 61 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h"
struct psref_target {
 struct psref_class *prt_class;
 
# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h" 3 4
_Bool 
# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h"
       prt_draining;
};
# 75 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h"
struct psref {
 struct { struct psref *sle_next; } psref_entry;
 void *psref_debug;
 const struct psref_target *psref_target;
 struct lwp *psref_lwp;
 struct cpu_info *psref_cpu;
};


void psref_init(void);

struct psref_class *
 psref_class_create(const char *, int);
void psref_class_destroy(struct psref_class *);

void psref_target_init(struct psref_target *, struct psref_class *);
void psref_target_destroy(struct psref_target *, struct psref_class *);

void psref_acquire(struct psref *, const struct psref_target *,
     struct psref_class *);
void psref_release(struct psref *, const struct psref_target *,
     struct psref_class *);
void psref_copy(struct psref *, const struct psref *,
     struct psref_class *);



# 101 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h" 3 4
_Bool 
# 101 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/psref.h"
    psref_held(const struct psref_target *, struct psref_class *);
# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 2
# 65 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
struct route {
 struct rtentry *_ro_rt;
 struct sockaddr *ro_sa;
 uint64_t ro_rtcache_generation;
 struct psref ro_psref;
 int ro_bound;
};





struct rt_metrics {
 uint64_t rmx_locks;
 uint64_t rmx_mtu;
 uint64_t rmx_hopcount;
 uint64_t rmx_recvpipe;
 uint64_t rmx_sendpipe;
 uint64_t rmx_ssthresh;
 uint64_t rmx_rtt;
 uint64_t rmx_rttvar;
 time_t rmx_expire;
 time_t rmx_pksent;
};
# 107 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/radix.h" 1
# 41 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/radix.h"
struct radix_node {
 struct radix_mask *rn_mklist;
 struct radix_node *rn_p;
 short rn_b;
 char rn_bmask;
 u_char rn_flags;



 union {
  struct {
   const char *rn_Key;
   const char *rn_Mask;
   struct radix_node *rn_Dupedkey;
  } rn_leaf;
  struct {
   int rn_Off;
   struct radix_node *rn_L;
   struct radix_node *rn_R;
  } rn_node;
 } rn_u;





};
# 80 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/radix.h"
struct radix_mask {
 short rm_b;
 char rm_unused;
 u_char rm_flags;
 struct radix_mask *rm_mklist;
 union {
  const char *rmu_mask;
  struct radix_node *rmu_leaf;
 } rm_rmu;
 int rm_refs;
};
# 104 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/radix.h"
struct radix_node_head {
 struct radix_node *rnh_treetop;
 int rnh_addrsize;
 int rnh_pktsize;
 struct radix_node *(*rnh_addaddr)
  (const void *v, const void *mask,
       struct radix_node_head *head, struct radix_node nodes[]);
 struct radix_node *(*rnh_addpkt)
  (const void *v, const void *mask,
       struct radix_node_head *head, struct radix_node nodes[]);
 struct radix_node *(*rnh_deladdr)
  (const void *v, const void *mask, struct radix_node_head *head);
 struct radix_node *(*rnh_delpkt)
  (const void *v, const void *mask, struct radix_node_head *head);
 struct radix_node *(*rnh_matchaddr)
  (const void *v, struct radix_node_head *head);
 struct radix_node *(*rnh_lookup)
  (const void *v, const void *mask, struct radix_node_head *head);
 struct radix_node *(*rnh_matchpkt)
  (const void *v, struct radix_node_head *head);
 struct radix_node rnh_nodes[3];
};


extern struct radix_mask *rn_mkfreelist;





void rn_init(void);
int rn_inithead(void **, int);
void rn_delayedinit(void **, int);
int rn_inithead0(struct radix_node_head *, int);
int rn_refines(const void *, const void *);
int rn_walktree(struct radix_node_head *,
             int (*)(struct radix_node *, void *),
      void *);
struct radix_node *
 rn_search_matched(struct radix_node_head *,
                   int (*)(struct radix_node *, void *),
            void *);
struct radix_node
  *rn_addmask(const void *, int, int),
  *rn_addroute(const void *, const void *, struct radix_node_head *,
   struct radix_node [2]),
  *rn_delete1(const void *, const void *, struct radix_node_head *,
   struct radix_node *),
  *rn_delete(const void *, const void *, struct radix_node_head *),
  *rn_insert(const void *, struct radix_node_head *, int *,
   struct radix_node [2]),
  *rn_lookup(const void *, const void *, struct radix_node_head *),
  *rn_match(const void *, struct radix_node_head *),
  *rn_newpair(const void *, int, struct radix_node[2]),
  *rn_search(const void *, struct radix_node *),
  *rn_search_m(const void *, struct radix_node *, const void *);
# 108 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 2

struct rtentry {
 struct radix_node rt_nodes[2];

 struct sockaddr *rt_gateway;
 int rt_flags;
 int rt_refcnt;
 uint64_t rt_use;
 struct ifnet *rt_ifp;
 struct ifaddr *rt_ifa;
 uint32_t rt_ifa_seqno;
 void * rt_llinfo;
 struct rt_metrics rt_rmx;
 struct rtentry *rt_gwroute;
 struct { struct rttimer *lh_first; } rt_timer;
 struct rtentry *rt_parent;
 struct sockaddr *_rt_key;
 struct sockaddr *rt_tag;

 kcondvar_t rt_cv;
 struct psref_target rt_psref;
 struct { struct rtentry *sle_next; } rt_free;

};

static __inline const struct sockaddr *
rt_getkey(const struct rtentry *rt)
{
 return rt->_rt_key;
}





struct ortentry {
 uint32_t rt_hash;
 struct sockaddr rt_dst;
 struct sockaddr rt_gateway;
 int16_t rt_flags;
 int16_t rt_refcnt;
 uint32_t rt_use;
 struct ifnet *rt_ifp;
};
# 195 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
struct rtstat {
 uint64_t rts_badredirect;
 uint64_t rts_dynamic;
 uint64_t rts_newgateway;
 uint64_t rts_unreach;
 uint64_t rts_wildcard;
};
# 219 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
struct rt_msghdr {
 u_short rtm_msglen __attribute__((__aligned__(sizeof(uint64_t))));

 u_char rtm_version;
 u_char rtm_type;
 u_short rtm_index;
 int rtm_flags;
 int rtm_addrs;
 __pid_t rtm_pid;
 int rtm_seq;
 int rtm_errno;
 int rtm_use;
 int rtm_inits;
 struct rt_metrics rtm_rmx __attribute__((__aligned__(sizeof(uint64_t))));

};
# 333 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
struct rt_addrinfo {
 int rti_addrs;
 const struct sockaddr *rti_info[9];
 int rti_flags;
 struct ifaddr *rti_ifa;
 struct ifnet *rti_ifp;
};

struct route_cb {
 int ip_count;
 int ip6_count;
 int unused1;
 int mpls_count;
 int any_count;
};







struct rttimer {
 struct { struct rttimer *tqe_next; struct rttimer * *tqe_prev; } rtt_next;
 struct { struct rttimer *le_next; struct rttimer **le_prev; } rtt_link;
 struct rttimer_queue *rtt_queue;
 struct rtentry *rtt_rt;
 void (*rtt_func)(struct rtentry *, struct rttimer *);
 time_t rtt_time;
};

struct rttimer_queue {
 long rtq_timeout;
 unsigned long rtq_count;
 struct { struct rttimer *tqh_first; struct rttimer * *tqh_last; } rtq_head;
 struct { struct rttimer_queue *le_next; struct rttimer_queue **le_prev; } rtq_link;
};


struct rtbl;
typedef struct rtbl rtbl_t;



struct rtbl {
 struct radix_node_head t_rnh;
};

struct rt_walkarg {
 int w_op;
 int w_arg;
 int w_given;
 int w_needed;
 void * w_where;
 int w_tmemsize;
 int w_tmemneeded;
 void * w_tmem;
};







struct rtwalk {
 int (*rw_f)(struct rtentry *, void *);
 void *rw_v;
};




struct route_info {
 struct sockaddr ri_dst;
 struct sockaddr ri_src;
 struct route_cb ri_cb;
 int ri_maxqlen;
 struct ifqueue ri_intrq;
 void *ri_sih;
};

extern struct route_info route_info;
extern struct rtstat rtstat;

struct socket;

void rt_init(void);

int rt_timer_add(struct rtentry *,
     void(*)(struct rtentry *, struct rttimer *),
     struct rttimer_queue *);
unsigned long
 rt_timer_count(struct rttimer_queue *);
void rt_timer_queue_change(struct rttimer_queue *, long);
struct rttimer_queue *
 rt_timer_queue_create(u_int);
void rt_timer_queue_destroy(struct rttimer_queue *);

void rt_free(struct rtentry *);
void rt_unref(struct rtentry *);

int rt_update(struct rtentry *, struct rt_addrinfo *, void *);
int rt_update_prepare(struct rtentry *);
void rt_update_finish(struct rtentry *);

void rt_newmsg(const int, const struct rtentry *);
void rt_newmsg_dynamic(const int, const struct rtentry *);
struct rtentry *
 rtalloc1(const struct sockaddr *, int);
int rtinit(struct ifaddr *, int, int);
void rtredirect(const struct sockaddr *, const struct sockaddr *,
     const struct sockaddr *, int, const struct sockaddr *,
     struct rtentry **);
int rtrequest(int, const struct sockaddr *,
     const struct sockaddr *, const struct sockaddr *, int,
     struct rtentry **);
int rtrequest1(int, struct rt_addrinfo *, struct rtentry **);

int rt_ifa_addlocal(struct ifaddr *);
int rt_ifa_remlocal(struct ifaddr *, struct ifaddr *);
struct ifaddr *
 rt_get_ifa(struct rtentry *);
void rt_replace_ifa(struct rtentry *, struct ifaddr *);
int rt_setgate(struct rtentry *, const struct sockaddr *);

const struct sockaddr *
 rt_settag(struct rtentry *, const struct sockaddr *);
struct sockaddr *
 rt_gettag(const struct rtentry *);

int rt_check_reject_route(const struct rtentry *, const struct ifnet *);
void rt_delete_matched_entries(sa_family_t,
     int (*)(struct rtentry *, void *), void *, 
# 466 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 3 4
                                               _Bool
# 466 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
                                                   );
void rt_replace_ifa_matched_entries(sa_family_t,
     int (*)(struct rtentry *, void *), void *, struct ifaddr *);
int rt_walktree(sa_family_t, int (*)(struct rtentry *, void *), void *);

static __inline void
rt_assert_referenced(const struct rtentry *rt)
{

 ((void)sizeof((long)(rt->rt_refcnt > 0)));
}

void rtcache_copy(struct route *, struct route *);
void rtcache_free(struct route *);
struct rtentry *
 rtcache_init(struct route *);
struct rtentry *
 rtcache_init_noclone(struct route *);
struct rtentry *
 rtcache_lookup2(struct route *, const struct sockaddr *, int,
     int *);
int rtcache_setdst(struct route *, const struct sockaddr *);
struct rtentry *
 rtcache_update(struct route *, int);

static __inline void
rtcache_invariants(const struct route *ro)
{

 ((void)sizeof((long)(ro->ro_sa != 
# 495 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 3 4
((void *)0) 
# 495 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
|| ro->_ro_rt == 
# 495 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h" 3 4
((void *)0)
# 495 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/route.h"
)));
}

static __inline struct rtentry *
rtcache_lookup1(struct route *ro, const struct sockaddr *dst, int clone)
{
 int hit;

 return rtcache_lookup2(ro, dst, clone, &hit);
}

static __inline struct rtentry *
rtcache_lookup(struct route *ro, const struct sockaddr *dst)
{
 return rtcache_lookup1(ro, dst, 1);
}

static __inline const struct sockaddr *
rtcache_getdst(const struct route *ro)
{

 rtcache_invariants(ro);
 return ro->ro_sa;
}

struct rtentry *
 rtcache_validate(struct route *);

void rtcache_unref(struct rtentry *, struct route *);

percpu_t *
 rtcache_percpu_alloc(void);

static __inline struct route *
rtcache_percpu_getref(percpu_t *pc)
{

 return *(struct route **)percpu_getref(pc);
}

static __inline void
rtcache_percpu_putref(percpu_t *pc)
{

 percpu_putref(pc);
}



void rt_ieee80211msg(struct ifnet *, int, void *, size_t);
void rt_ifannouncemsg(struct ifnet *, int);
void rt_ifmsg(struct ifnet *);
void rt_missmsg(int, const struct rt_addrinfo *, int, int);
struct mbuf *
 rt_msg1(int, struct rt_addrinfo *, void *, int);
int rt_msg3(int, struct rt_addrinfo *, void *, struct rt_walkarg *, int *);
void rt_addrmsg(int, struct ifaddr *);
void rt_addrmsg_src(int, struct ifaddr *, const struct sockaddr *);
void rt_addrmsg_rt(int, struct ifaddr *, int, struct rtentry *);
void route_enqueue(struct mbuf *, int);

struct llentry;
void rt_clonedmsg(int, const struct sockaddr *, const struct sockaddr *,
     const uint8_t *, const struct ifnet *);

void rt_setmetrics(void *, struct rtentry *);


int rt_addaddr(rtbl_t *, struct rtentry *, const struct sockaddr *);
void rt_assert_inactive(const struct rtentry *);
struct rtentry *
 rt_deladdr(rtbl_t *, const struct sockaddr *,
     const struct sockaddr *);
rtbl_t *rt_gettable(sa_family_t);
int rt_inithead(rtbl_t **, int);
struct rtentry *
 rt_lookup(rtbl_t *, const struct sockaddr *,
     const struct sockaddr *);
struct rtentry *
 rt_matchaddr(rtbl_t *, const struct sockaddr *);
int rt_refines(const struct sockaddr *, const struct sockaddr *);
int rtbl_walktree(sa_family_t, int (*)(struct rtentry *, void *), void *);
struct rtentry *
 rtbl_search_matched_entry(sa_family_t,
     int (*)(struct rtentry *, void *), void *);
void rtbl_init(void);

void sysctl_net_route_setup(struct sysctllog **, int, const char *);

void rt_unhandled(const char *, const struct ifnet *,
    const struct sockaddr *);
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/domain.h" 2




struct lwp;
struct mbuf;
struct ifnet;
struct ifqueue;
struct sockaddr;

struct dom_rtlist { struct route *lh_first; };

struct domain {
 int dom_family;
 const char *dom_name;
 void (*dom_init)
   (void);
 int (*dom_externalize)
   (struct mbuf *, struct lwp *, int);
 void (*dom_dispose)
   (struct mbuf *);
 const struct protosw *dom_protosw, *dom_protoswNPROTOSW;
 int (*dom_rtattach)
   (rtbl_t **, int);
 int dom_rtoffset;
 int dom_maxrtkey;
 void (*dom_if_up)
   (struct ifnet *);
 void (*dom_if_down)
   (struct ifnet *);
 void *(*dom_ifattach)
   (struct ifnet *);
 void (*dom_ifdetach)
   (struct ifnet *, void *);
 void (*dom_if_link_state_change)
   (struct ifnet *, int);
 const void *(*dom_sockaddr_const_addr)(const struct sockaddr *,
            socklen_t *);
 void *(*dom_sockaddr_addr)(struct sockaddr *, socklen_t *);
 int (*dom_sockaddr_cmp)(const struct sockaddr *,
                             const struct sockaddr *);
 struct sockaddr *(*dom_sockaddr_externalize)(struct sockaddr *,
                                              socklen_t,
           const struct sockaddr *);
 const struct sockaddr *dom_sa_any;
 struct ifqueue *dom_ifqueues[2];
 struct { struct domain *stqe_next; } dom_link;
 struct mowner dom_mowner;
 uint_fast8_t dom_sa_cmpofs;
 uint_fast8_t dom_sa_cmplen;
};

struct domainhead { struct domain *stqh_first; struct domain **stqh_last; };







extern struct domainhead domains;
void domain_attach(struct domain *);
void domaininit(
# 105 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/domain.h" 3 4
               _Bool
# 105 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/domain.h"
                   );
void domaininit_post(void);
# 112 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/kernel.h" 1
# 114 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/ioctl.h" 1
# 115 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/sysctl.h" 1
# 10 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/sys/sysctl.h"
struct sysctl_oid {
    char oid_name[32];
    int oid_number;
};

extern struct sysctl_oid sysctl___vfs;
static inline void sysctl_wlock(void) {}
static inline void sysctl_wunlock(void) {}
static inline void sysctl_unregister_oid(struct sysctl_oid *oidp) {}
static inline void sysctl_register_oid(struct sysctl_oid *oidp) {}
# 116 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslog.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslog.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 39 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslog.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdarg.h" 1
# 39 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdarg.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 40 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdarg.h" 2
# 53 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stdarg.h"
typedef __va_list va_list;
# 41 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslog.h" 2
# 240 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/syslog.h"
void logpri(int);
void log(int, const char *, ...) __attribute__((__format__ (__printf__, 2, 3)));
void vlog(int, const char *, __va_list) __attribute__((__format__ (__printf__, 2, 0)));
void addlog(const char *, ...) __attribute__((__format__ (__printf__, 1, 2)));
void logwakeup(void);
# 117 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kauth.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kauth.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/secmodel/secmodel.h" 1
# 34 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/secmodel/secmodel.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/prop/proplib.h" 1




typedef void * prop_dictionary_t;
typedef void * prop_object_t;
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/secmodel/secmodel.h" 2

void secmodel_init(void);





typedef int (*secmodel_eval_t)(const char *, void *, void *);
typedef int (*secmodel_setinfo_t)(void *);




struct secmodel_descr {
 struct { struct secmodel_descr *le_next; struct secmodel_descr **le_prev; } sm_list;
 const char *sm_id;
 const char *sm_name;
 prop_dictionary_t sm_behavior;
 secmodel_eval_t sm_eval;
 secmodel_setinfo_t sm_setinfo;
};
typedef struct secmodel_descr *secmodel_t;

int secmodel_register(secmodel_t *, const char *, const char *,
    prop_dictionary_t, secmodel_eval_t, secmodel_setinfo_t);
int secmodel_deregister(secmodel_t);
int secmodel_nsecmodels(void);

int secmodel_eval(const char *, const char *, void *, void *);
int secmodel_setinfo(const char *, void *, int *);
# 39 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kauth.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stat.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stat.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stat.h" 2
# 56 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stat.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 2






struct timeval {
 time_t tv_sec;
 suseconds_t tv_usec;
};

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timespec.h" 1
# 47 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timespec.h"
struct timespec {
 time_t tv_sec;
 long tv_nsec;
};
# 50 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 2
# 69 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
struct timezone {
 int tz_minuteswest;
 int tz_dsttime;
};
# 106 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
struct bintime {
 time_t sec;
 uint64_t frac;
};

static __inline void
bintime_addx(struct bintime *bt, uint64_t x)
{
 uint64_t u;

 u = bt->frac;
 bt->frac += x;
 if (u > bt->frac)
  bt->sec++;
}

static __inline void
bintime_add(struct bintime *bt, const struct bintime *bt2)
{
 uint64_t u;

 u = bt->frac;
 bt->frac += bt2->frac;
 if (u > bt->frac)
  bt->sec++;
 bt->sec += bt2->sec;
}

static __inline void
bintime_sub(struct bintime *bt, const struct bintime *bt2)
{
 uint64_t u;

 u = bt->frac;
 bt->frac -= bt2->frac;
 if (u < bt->frac)
  bt->sec--;
 bt->sec -= bt2->sec;
}
# 178 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
static __inline void
bintime2timespec(const struct bintime *bt, struct timespec *ts)
{

 ts->tv_sec = bt->sec;
 ts->tv_nsec =
     (long)((1000000000ULL * (uint32_t)(bt->frac >> 32)) >> 32);
}

static __inline void
timespec2bintime(const struct timespec *ts, struct bintime *bt)
{

 bt->sec = ts->tv_sec;
 bt->frac = (uint64_t)ts->tv_nsec * ((uint64_t)18446744073ULL);
}

static __inline void
bintime2timeval(const struct bintime *bt, struct timeval *tv)
{

 tv->tv_sec = bt->sec;
 tv->tv_usec =
     (suseconds_t)((1000000ULL * (uint32_t)(bt->frac >> 32)) >> 32);
}

static __inline void
timeval2bintime(const struct timeval *tv, struct bintime *bt)
{

 bt->sec = tv->tv_sec;
 bt->frac = (uint64_t)tv->tv_usec * ((uint64_t)18446744073709ULL);
}

static __inline struct bintime
ms2bintime(uint64_t ms)
{
 struct bintime bt;

 bt.sec = (time_t)(ms / 1000U);
 bt.frac = (uint64_t)(ms % 1000U) * ((uint64_t)18446744073709551ULL);

 return bt;
}

static __inline struct bintime
us2bintime(uint64_t us)
{
 struct bintime bt;

 bt.sec = (time_t)(us / 1000000U);
 bt.frac = (uint64_t)(us % 1000000U) * ((uint64_t)18446744073709ULL);

 return bt;
}

static __inline struct bintime
ns2bintime(uint64_t ns)
{
 struct bintime bt;

 bt.sec = (time_t)(ns / 1000000000U);
 bt.frac = (uint64_t)(ns % 1000000000U) * ((uint64_t)18446744073ULL);

 return bt;
}
# 275 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"

# 275 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 3 4
_Bool 
# 275 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
    timespecaddok(const struct timespec *, const struct timespec *) __attribute__((__pure__));

# 276 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 3 4
_Bool 
# 276 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
    timespecsubok(const struct timespec *, const struct timespec *) __attribute__((__pure__));
# 291 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
struct itimerval {
 struct timeval it_interval;
 struct timeval it_value;
};





struct itimerspec {
 struct timespec it_interval;
 struct timespec it_value;
};
# 318 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timearith.h" 1
# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timearith.h"
struct itimerspec;
struct timespec;
struct timeval;

int tstohz(const struct timespec *);
int tvtohz(const struct timeval *);

int itimerfix(struct timeval *);
int itimespecfix(struct timespec *);

void itimer_transition(const struct itimerspec *restrict,
     const struct timespec *restrict,
     struct timespec *restrict, int *restrict);
# 319 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 1
# 69 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
struct itimer;
struct itlist { struct itimer *lh_first; };
# 86 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
struct itimer_ops {
 void (*ito_fire)(struct itimer *);
 void (*ito_realtime_changed)(struct itimer *);
};




struct itimer {
 union {
  struct {
   callout_t it_ch;
   struct { struct itimer *le_next; struct itimer **le_prev; } it_rtchgq;
  } it_real;
  struct {
   struct itlist *it_vlist;
   struct { struct itimer *le_next; struct itimer **le_prev; } it_list;
   
# 103 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
  _Bool 
# 103 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
         it_active;
  } it_virtual;
 };
 const struct itimer_ops *it_ops;
 struct itimerspec it_time;
 clockid_t it_clockid;
 int it_overruns;
 
# 110 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
_Bool 
# 110 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
     it_dying;
};
# 123 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
struct ptimer {
 struct itimer pt_itimer;

 struct { struct ptimer *tqe_next; struct ptimer * *tqe_prev; } pt_chain;
 struct sigevent pt_ev;
 int pt_poverruns;
 int pt_entry;
 struct proc *pt_proc;
 
# 131 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
_Bool 
# 131 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
     pt_queued;
};







struct ptimers {
 struct itlist pts_virtual;
 struct itlist pts_prof;
 struct itimer *pts_timers[36];
};
# 169 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
void binuptime(struct bintime *);
void nanouptime(struct timespec *);
void microuptime(struct timeval *);

void bintime(struct bintime *);
void nanotime(struct timespec *);
void microtime(struct timeval *);

void getbinuptime(struct bintime *);
void getnanouptime(struct timespec *);
void getmicrouptime(struct timeval *);

void getbintime(struct bintime *);
void getnanotime(struct timespec *);
void getmicrotime(struct timeval *);

void getbinboottime(struct bintime *);
void getnanoboottime(struct timespec *);
void getmicroboottime(struct timeval *);


int ts2timo(clockid_t, int, struct timespec *, int *, struct timespec *);
void adjtime1(const struct timeval *, struct timeval *, struct proc *);
int clock_getres1(clockid_t, struct timespec *);
int clock_gettime1(clockid_t, struct timespec *);
int clock_settime1(struct proc *, clockid_t, const struct timespec *, 
# 194 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
                                                                     _Bool
# 194 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
                                                                         );
void clock_timeleft(clockid_t, struct timespec *, struct timespec *);
int dogetitimer(struct proc *, int, struct itimerval *);
int dosetitimer(struct proc *, int, struct itimerval *);
int dotimer_gettime(int, struct proc *, struct itimerspec *);
int dotimer_settime(int, struct itimerspec *, struct itimerspec *, int,
     struct proc *);
int tshzto(const struct timespec *);
int tshztoup(const struct timespec *);
int tvhzto(const struct timeval *);
void inittimecounter(void);
int ppsratecheck(struct timeval *, int *, int);
int ratecheck(struct timeval *, const struct timeval *);
int settime(struct proc *p, struct timespec *);
int nanosleep1(struct lwp *, clockid_t, int, struct timespec *,
     struct timespec *);
int settimeofday1(const struct timeval *, 
# 210 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
                                         _Bool
# 210 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
                                             ,
     const void *, struct lwp *, 
# 211 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
                                _Bool
# 211 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
                                    );
int timer_create1(timer_t *, clockid_t, struct sigevent *, copyin_t,
     struct lwp *);
int inittimeleft(struct timespec *, struct timespec *);
int gettimeleft(struct timespec *, struct timespec *);
void timerupcall(struct lwp *);
void time_init(void);

# 218 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
_Bool 
# 218 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
    time_wraps(struct timespec *, struct timespec *);

void itimer_init(struct itimer *, const struct itimer_ops *,
     clockid_t, struct itlist *);
void itimer_poison(struct itimer *);
void itimer_fini(struct itimer *);

void itimer_lock(void);
void itimer_unlock(void);

# 227 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
_Bool 
# 227 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
    itimer_lock_held(void);
int itimer_settime(struct itimer *);
void itimer_gettime(const struct itimer *, struct itimerspec *);

void ptimer_tick(struct lwp *, 
# 231 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h" 3 4
                              _Bool
# 231 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
                                  );
void ptimers_free(struct proc *, int);







extern volatile time_t time__second;
extern volatile time_t time__uptime;

static inline time_t
getrealtime(void)
{
 return ({ const volatile __typeof__(*(&time__second)) *__al_ptr = (&time__second); do { struct __ctassert3_struct { unsigned int __ctassert3 : (sizeof(*(__al_ptr)) <= 8) ? 1 : -1; }; ((void)sizeof((long)(((uintptr_t)(__al_ptr) & (sizeof(*(__al_ptr)) - 1)) == 0))); } while (0); const __typeof_unqual__(*(__al_ptr)) __al_val = *(__al_ptr); __al_val; });
}

static inline time_t
getuptime(void)
{
 return ({ const volatile __typeof__(*(&time__uptime)) *__al_ptr = (&time__uptime); do { struct __ctassert4_struct { unsigned int __ctassert4 : (sizeof(*(__al_ptr)) <= 8) ? 1 : -1; }; ((void)sizeof((long)(((uintptr_t)(__al_ptr) & (sizeof(*(__al_ptr)) - 1)) == 0))); } while (0); const __typeof_unqual__(*(__al_ptr)) __al_val = *(__al_ptr); __al_val; });
}

static inline time_t
getboottime(void)
{
 return getrealtime() - getuptime();
}

static inline uint32_t
getuptime32(void)
{
 return getuptime() & 0xffffffff;
}
# 276 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/timevar.h"
extern int time_adjusted;







static __inline time_t time_mono_to_wall(time_t t)
{

 return t + getboottime();
}

static __inline time_t time_wall_to_mono(time_t t)
{

 return t - getboottime();
}
# 320 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/time.h" 2
# 57 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stat.h" 2


struct stat {
 dev_t st_dev;
 __mode_t st_mode;
 ino_t st_ino;
 nlink_t st_nlink;
 __uid_t st_uid;
 __gid_t st_gid;
 dev_t st_rdev;


 struct timespec st_atim;
 struct timespec st_mtim;
 struct timespec st_ctim;
 struct timespec st_birthtim;
# 83 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/stat.h"
 __off_t st_size;
 blkcnt_t st_blocks;
 blksize_t st_blksize;
 uint32_t st_flags;
 uint32_t st_gen;
 uint32_t st_spare[2];
};
# 40 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kauth.h" 2

struct uucred;
struct ki_ucred;
struct ki_pcred;
struct proc;
struct tty;
struct vnode;
struct cwdinfo;

enum uio_seg;


typedef struct kauth_scope *kauth_scope_t;
typedef struct kauth_listener *kauth_listener_t;
typedef uint64_t kauth_action_t;
typedef int (*kauth_scope_callback_t)(kauth_cred_t, kauth_action_t,
          void *, void *, void *, void *, void *);
typedef struct kauth_key *kauth_key_t;
# 118 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kauth.h"
enum {
 KAUTH_GENERIC_UNUSED1=1,
 KAUTH_GENERIC_ISSUSER,
};




enum {
 KAUTH_SYSTEM_ACCOUNTING=1,
 KAUTH_SYSTEM_CHROOT,
 KAUTH_SYSTEM_CHSYSFLAGS,
 KAUTH_SYSTEM_CPU,
 KAUTH_SYSTEM_DEBUG,
 KAUTH_SYSTEM_FILEHANDLE,
 KAUTH_SYSTEM_MKNOD,
 KAUTH_SYSTEM_MOUNT,
 KAUTH_SYSTEM_PSET,
 KAUTH_SYSTEM_REBOOT,
 KAUTH_SYSTEM_SETIDCORE,
 KAUTH_SYSTEM_SWAPCTL,
 KAUTH_SYSTEM_SYSCTL,
 KAUTH_SYSTEM_TIME,
 KAUTH_SYSTEM_MODULE,
 KAUTH_SYSTEM_FS_RESERVEDSPACE,
 KAUTH_SYSTEM_FS_QUOTA,
 KAUTH_SYSTEM_SEMAPHORE,
 KAUTH_SYSTEM_SYSVIPC,
 KAUTH_SYSTEM_MQUEUE,
 KAUTH_SYSTEM_VERIEXEC,
 KAUTH_SYSTEM_DEVMAPPER,
 KAUTH_SYSTEM_MAP_VA_ZERO,
 KAUTH_SYSTEM_LFS,
 KAUTH_SYSTEM_FS_EXTATTR,
 KAUTH_SYSTEM_FS_SNAPSHOT,
 KAUTH_SYSTEM_INTR,
 KAUTH_SYSTEM_KERNADDR,
};




enum kauth_system_req {
 KAUTH_REQ_SYSTEM_CHROOT_CHROOT=1,
 KAUTH_REQ_SYSTEM_CHROOT_FCHROOT,
 KAUTH_REQ_SYSTEM_CPU_SETSTATE,
 KAUTH_REQ_SYSTEM_MOUNT_GET,
 KAUTH_REQ_SYSTEM_MOUNT_NEW,
 KAUTH_REQ_SYSTEM_MOUNT_UNMOUNT,
 KAUTH_REQ_SYSTEM_MOUNT_UPDATE,
 KAUTH_REQ_SYSTEM_PSET_ASSIGN,
 KAUTH_REQ_SYSTEM_PSET_BIND,
 KAUTH_REQ_SYSTEM_PSET_CREATE,
 KAUTH_REQ_SYSTEM_PSET_DESTROY,
 KAUTH_REQ_SYSTEM_SYSCTL_ADD,
 KAUTH_REQ_SYSTEM_SYSCTL_DELETE,
 KAUTH_REQ_SYSTEM_SYSCTL_DESC,
 KAUTH_REQ_SYSTEM_SYSCTL_MODIFY,
 KAUTH_REQ_SYSTEM_SYSCTL_PRVT,
 KAUTH_REQ_SYSTEM_TIME_ADJTIME,
 KAUTH_REQ_SYSTEM_TIME_NTPADJTIME,
 KAUTH_REQ_SYSTEM_TIME_RTCOFFSET,
 KAUTH_REQ_SYSTEM_TIME_SYSTEM,
 KAUTH_REQ_SYSTEM_TIME_TIMECOUNTERS,
 KAUTH_REQ_SYSTEM_FS_QUOTA_GET,
 KAUTH_REQ_SYSTEM_FS_QUOTA_MANAGE,
 KAUTH_REQ_SYSTEM_FS_QUOTA_NOLIMIT,
 KAUTH_REQ_SYSTEM_FS_QUOTA_ONOFF,
 KAUTH_REQ_SYSTEM_SYSVIPC_BYPASS,
 KAUTH_REQ_SYSTEM_SYSVIPC_SHM_LOCK,
 KAUTH_REQ_SYSTEM_SYSVIPC_SHM_UNLOCK,
 KAUTH_REQ_SYSTEM_SYSVIPC_MSGQ_OVERSIZE,
 KAUTH_REQ_SYSTEM_VERIEXEC_ACCESS,
 KAUTH_REQ_SYSTEM_VERIEXEC_MODIFY,
 KAUTH_REQ_SYSTEM_LFS_MARKV,
 KAUTH_REQ_SYSTEM_LFS_BMAPV,
 KAUTH_REQ_SYSTEM_LFS_SEGCLEAN,
 KAUTH_REQ_SYSTEM_LFS_SEGWAIT,
 KAUTH_REQ_SYSTEM_LFS_FCNTL,
 KAUTH_REQ_SYSTEM_MOUNT_UMAP,
 KAUTH_REQ_SYSTEM_MOUNT_DEVICE,
 KAUTH_REQ_SYSTEM_INTR_AFFINITY,
};




enum {
 KAUTH_PROCESS_CANSEE=1,
 KAUTH_PROCESS_CORENAME,
 KAUTH_PROCESS_FORK,
 KAUTH_PROCESS_KEVENT_FILTER,
 KAUTH_PROCESS_KTRACE,
 KAUTH_PROCESS_NICE,
 KAUTH_PROCESS_PROCFS,
 KAUTH_PROCESS_PTRACE,
 KAUTH_PROCESS_RLIMIT,
 KAUTH_PROCESS_SCHEDULER_GETAFFINITY,
 KAUTH_PROCESS_SCHEDULER_SETAFFINITY,
 KAUTH_PROCESS_SCHEDULER_GETPARAM,
 KAUTH_PROCESS_SCHEDULER_SETPARAM,
 KAUTH_PROCESS_SETID,
 KAUTH_PROCESS_SIGNAL,
 KAUTH_PROCESS_STOPFLAG
};




enum kauth_process_req {
 KAUTH_REQ_PROCESS_CANSEE_ARGS=1,
 KAUTH_REQ_PROCESS_CANSEE_ENTRY,
 KAUTH_REQ_PROCESS_CANSEE_ENV,
 KAUTH_REQ_PROCESS_CANSEE_OPENFILES,
 KAUTH_REQ_PROCESS_CORENAME_GET,
 KAUTH_REQ_PROCESS_CORENAME_SET,
 KAUTH_REQ_PROCESS_KTRACE_PERSISTENT,
 KAUTH_REQ_PROCESS_PROCFS_READ,
 KAUTH_REQ_PROCESS_PROCFS_RW,
 KAUTH_REQ_PROCESS_PROCFS_WRITE,
 KAUTH_REQ_PROCESS_RLIMIT_GET,
 KAUTH_REQ_PROCESS_RLIMIT_SET,
 KAUTH_REQ_PROCESS_RLIMIT_BYPASS,
 KAUTH_REQ_PROCESS_CANSEE_EPROC,
 KAUTH_REQ_PROCESS_CANSEE_KPTR
};




enum {
 KAUTH_NETWORK_ALTQ=1,
 KAUTH_NETWORK_BIND,
 KAUTH_NETWORK_FIREWALL,
 KAUTH_NETWORK_INTERFACE,
 KAUTH_NETWORK_FORWSRCRT,
 KAUTH_NETWORK_NFS,
 KAUTH_NETWORK_ROUTE,
 KAUTH_NETWORK_SOCKET,
 KAUTH_NETWORK_INTERFACE_PPP,
 KAUTH_NETWORK_INTERFACE_SLIP,
 KAUTH_NETWORK_INTERFACE_STRIP,
 KAUTH_NETWORK_INTERFACE_TUN,
 KAUTH_NETWORK_INTERFACE_BRIDGE,
 KAUTH_NETWORK_IPSEC,
 KAUTH_NETWORK_INTERFACE_PVC,
 KAUTH_NETWORK_IPV6,
 KAUTH_NETWORK_SMB,
 KAUTH_NETWORK_INTERFACE_WG,
};




enum kauth_network_req {
 KAUTH_REQ_NETWORK_ALTQ_AFMAP=1,
 KAUTH_REQ_NETWORK_ALTQ_BLUE,
 KAUTH_REQ_NETWORK_ALTQ_CBQ,
 KAUTH_REQ_NETWORK_ALTQ_CDNR,
 KAUTH_REQ_NETWORK_ALTQ_CONF,
 KAUTH_REQ_NETWORK_ALTQ_FIFOQ,
 KAUTH_REQ_NETWORK_ALTQ_HFSC,
 KAUTH_REQ_NETWORK_ALTQ_JOBS,
 KAUTH_REQ_NETWORK_ALTQ_PRIQ,
 KAUTH_REQ_NETWORK_ALTQ_RED,
 KAUTH_REQ_NETWORK_ALTQ_RIO,
 KAUTH_REQ_NETWORK_ALTQ_WFQ,
 KAUTH_REQ_NETWORK_BIND_PORT,
 KAUTH_REQ_NETWORK_BIND_PRIVPORT,
 KAUTH_REQ_NETWORK_FIREWALL_FW,
 KAUTH_REQ_NETWORK_FIREWALL_NAT,
 KAUTH_REQ_NETWORK_INTERFACE_GET,
 KAUTH_REQ_NETWORK_INTERFACE_GETPRIV,
 KAUTH_REQ_NETWORK_INTERFACE_SET,
 KAUTH_REQ_NETWORK_INTERFACE_SETPRIV,
 KAUTH_REQ_NETWORK_NFS_EXPORT,
 KAUTH_REQ_NETWORK_NFS_SVC,
 KAUTH_REQ_NETWORK_SOCKET_OPEN,
 KAUTH_REQ_NETWORK_SOCKET_RAWSOCK,
 KAUTH_REQ_NETWORK_SOCKET_CANSEE,
 KAUTH_REQ_NETWORK_SOCKET_DROP,
 KAUTH_REQ_NETWORK_SOCKET_SETPRIV,
 KAUTH_REQ_NETWORK_INTERFACE_PPP_ADD,
 KAUTH_REQ_NETWORK_INTERFACE_SLIP_ADD,
 KAUTH_REQ_NETWORK_INTERFACE_STRIP_ADD,
 KAUTH_REQ_NETWORK_INTERFACE_TUN_ADD,
 KAUTH_REQ_NETWORK_IPV6_HOPBYHOP,
 KAUTH_REQ_NETWORK_INTERFACE_BRIDGE_GETPRIV,
 KAUTH_REQ_NETWORK_INTERFACE_BRIDGE_SETPRIV,
 KAUTH_REQ_NETWORK_IPSEC_BYPASS,
 KAUTH_REQ_NETWORK_IPV6_JOIN_MULTICAST,
 KAUTH_REQ_NETWORK_INTERFACE_PVC_ADD,
 KAUTH_REQ_NETWORK_SMB_SHARE_ACCESS,
 KAUTH_REQ_NETWORK_SMB_SHARE_CREATE,
 KAUTH_REQ_NETWORK_SMB_VC_ACCESS,
 KAUTH_REQ_NETWORK_SMB_VC_CREATE,
 KAUTH_REQ_NETWORK_INTERFACE_FIRMWARE,
 KAUTH_REQ_NETWORK_BIND_ANYADDR,
 KAUTH_REQ_NETWORK_INTERFACE_WG_GETPRIV,
 KAUTH_REQ_NETWORK_INTERFACE_WG_SETPRIV,
};




enum {
 KAUTH_MACHDEP_CACHEFLUSH=1,
 KAUTH_MACHDEP_CPU_UCODE_APPLY,
 KAUTH_MACHDEP_IOPERM_GET,
 KAUTH_MACHDEP_IOPERM_SET,
 KAUTH_MACHDEP_IOPL,
 KAUTH_MACHDEP_LDT_GET,
 KAUTH_MACHDEP_LDT_SET,
 KAUTH_MACHDEP_MTRR_GET,
 KAUTH_MACHDEP_MTRR_SET,
 KAUTH_MACHDEP_NVRAM,
 KAUTH_MACHDEP_UNMANAGEDMEM,
 KAUTH_MACHDEP_PXG,
 KAUTH_MACHDEP_SVS_DISABLE
};




enum {
 KAUTH_DEVICE_TTY_OPEN=1,
 KAUTH_DEVICE_TTY_PRIVSET,
 KAUTH_DEVICE_TTY_STI,
 KAUTH_DEVICE_RAWIO_SPEC,
 KAUTH_DEVICE_RAWIO_PASSTHRU,
 KAUTH_DEVICE_BLUETOOTH_SETPRIV,
 KAUTH_DEVICE_RND_ADDDATA,
 KAUTH_DEVICE_RND_ADDDATA_ESTIMATE,
 KAUTH_DEVICE_RND_GETPRIV,
 KAUTH_DEVICE_RND_SETPRIV,
 KAUTH_DEVICE_BLUETOOTH_BCSP,
 KAUTH_DEVICE_BLUETOOTH_BTUART,
 KAUTH_DEVICE_GPIO_PINSET,
 KAUTH_DEVICE_BLUETOOTH_SEND,
 KAUTH_DEVICE_BLUETOOTH_RECV,
 KAUTH_DEVICE_TTY_VIRTUAL,
 KAUTH_DEVICE_WSCONS_KEYBOARD_BELL,
 KAUTH_DEVICE_WSCONS_KEYBOARD_KEYREPEAT,
 KAUTH_DEVICE_NVMM_CTL,
};




enum kauth_device_req {
 KAUTH_REQ_DEVICE_RAWIO_SPEC_READ=1,
 KAUTH_REQ_DEVICE_RAWIO_SPEC_WRITE,
 KAUTH_REQ_DEVICE_RAWIO_SPEC_RW,
 KAUTH_REQ_DEVICE_BLUETOOTH_BCSP_ADD,
 KAUTH_REQ_DEVICE_BLUETOOTH_BTUART_ADD,
};




enum {
 KAUTH_CRED_INIT=1,
 KAUTH_CRED_FORK,
 KAUTH_CRED_COPY,
 KAUTH_CRED_FREE,
 KAUTH_CRED_CHROOT
};
# 460 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kauth.h"
void kauth_init(void);
kauth_scope_t kauth_register_scope(const char *, kauth_scope_callback_t, void *);
void kauth_deregister_scope(kauth_scope_t);
kauth_listener_t kauth_listen_scope(const char *, kauth_scope_callback_t, void *);
void kauth_unlisten_scope(kauth_listener_t);
int kauth_authorize_action(kauth_scope_t, kauth_cred_t, kauth_action_t, void *,
    void *, void *, void *);


int kauth_authorize_generic(kauth_cred_t, kauth_action_t, void *);
int kauth_authorize_system(kauth_cred_t, kauth_action_t, enum kauth_system_req,
    void *, void *, void *);
int kauth_authorize_process(kauth_cred_t, kauth_action_t, struct proc *,
    void *, void *, void *);
int kauth_authorize_network(kauth_cred_t, kauth_action_t,
    enum kauth_network_req, void *, void *, void *);
int kauth_authorize_machdep(kauth_cred_t, kauth_action_t,
    void *, void *, void *, void *);
int kauth_authorize_device(kauth_cred_t, kauth_action_t,
    void *, void *, void *, void *);
int kauth_authorize_device_tty(kauth_cred_t, kauth_action_t, struct tty *);
int kauth_authorize_device_spec(kauth_cred_t, enum kauth_device_req,
    struct vnode *);
int kauth_authorize_device_passthru(kauth_cred_t, dev_t, u_long, void *);
int kauth_authorize_vnode(kauth_cred_t, kauth_action_t, struct vnode *,
    struct vnode *, int);


kauth_cred_t kauth_cred_alloc(void);
void kauth_cred_free(kauth_cred_t);
void kauth_cred_clone(kauth_cred_t, kauth_cred_t);
kauth_cred_t kauth_cred_dup(kauth_cred_t);
kauth_cred_t kauth_cred_copy(kauth_cred_t);

__uid_t kauth_cred_getuid(kauth_cred_t);
__uid_t kauth_cred_geteuid(kauth_cred_t);
__uid_t kauth_cred_getsvuid(kauth_cred_t);
__gid_t kauth_cred_getgid(kauth_cred_t);
__gid_t kauth_cred_getegid(kauth_cred_t);
__gid_t kauth_cred_getsvgid(kauth_cred_t);
int kauth_cred_ismember_gid(kauth_cred_t, __gid_t, int *);
int kauth_cred_groupmember(kauth_cred_t, __gid_t);
u_int kauth_cred_ngroups(kauth_cred_t);
__gid_t kauth_cred_group(kauth_cred_t, u_int);

void kauth_cred_setuid(kauth_cred_t, __uid_t);
void kauth_cred_seteuid(kauth_cred_t, __uid_t);
void kauth_cred_setsvuid(kauth_cred_t, __uid_t);
void kauth_cred_setgid(kauth_cred_t, __gid_t);
void kauth_cred_setegid(kauth_cred_t, __gid_t);
void kauth_cred_setsvgid(kauth_cred_t, __gid_t);

kauth_cred_t kauth_cred_hold(kauth_cred_t);
u_int kauth_cred_getrefcnt(kauth_cred_t);

int kauth_cred_setgroups(kauth_cred_t, const __gid_t *, size_t, __uid_t,
    enum uio_seg);
int kauth_cred_getgroups(kauth_cred_t, __gid_t *, size_t, enum uio_seg);


int kauth_proc_setgroups(struct lwp *, kauth_cred_t);

int kauth_register_key(secmodel_t, kauth_key_t *);
int kauth_deregister_key(kauth_key_t);
void kauth_cred_setdata(kauth_cred_t, kauth_key_t, void *);
void *kauth_cred_getdata(kauth_cred_t, kauth_key_t);

int kauth_cred_uidmatch(kauth_cred_t, kauth_cred_t);
void kauth_uucred_to_cred(kauth_cred_t, const struct uucred *);
void kauth_cred_to_uucred(struct uucred *, const kauth_cred_t);
int kauth_cred_uucmp(kauth_cred_t, const struct uucred *);
void kauth_cred_toucred(kauth_cred_t, struct ki_ucred *);
void kauth_cred_topcred(kauth_cred_t, struct ki_pcred *);

kauth_action_t kauth_accmode_to_action(__accmode_t);
kauth_action_t kauth_extattr_action(__mode_t);





kauth_cred_t kauth_cred_get(void);

void kauth_proc_fork(struct proc *, struct proc *);
void kauth_proc_chroot(kauth_cred_t cred, struct cwdinfo *cwdi);
# 118 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kmem.h" 1
# 34 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kmem.h"
typedef unsigned int km_flag_t;

void kmem_init(void);
size_t kmem_roundup_size(size_t);

void * kmem_alloc(size_t, km_flag_t);
void * kmem_zalloc(size_t, km_flag_t);
void kmem_free(void *, size_t);

void * kmem_intr_alloc(size_t, km_flag_t);
void * kmem_intr_zalloc(size_t, km_flag_t);
void kmem_intr_free(void *, size_t);

char * kmem_asprintf(const char *, ...) __attribute__((__format__ (__printf__, 1, 2)));

char * kmem_strdupsize(const char *, size_t *, km_flag_t);

char * kmem_strndup(const char *, size_t, km_flag_t);
void kmem_strfree(char *);

void * kmem_tmpbuf_alloc(size_t, void *, size_t, km_flag_t);
void kmem_tmpbuf_free(void *, size_t, void *);
# 119 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/xcall.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/xcall.h"
typedef void (*xcfunc_t)(void *, void *);

struct cpu_info;

void xc_init_cpu(struct cpu_info *);
void xc_send_ipi(struct cpu_info *);
void xc_ipi_handler(void);

void xc__highpri_intr(void *);

uint64_t xc_broadcast(u_int, xcfunc_t, void *, void *);
uint64_t xc_unicast(u_int, xcfunc_t, void *, void *, struct cpu_info *);
void xc_wait(uint64_t);

void xc_barrier(u_int);

unsigned int xc_encode_ipl(int);
# 120 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/net/if.c" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cpu.h" 1
# 34 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cpu.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/src/bsd_compat/include/machine/cpu.h" 1
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/cpu.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h" 1
# 42 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 1
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
struct kcpuset;
typedef struct kcpuset kcpuset_t;



# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 1
# 73 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 74 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 2







struct sched_param {
 int sched_priority;
};
# 94 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"

# 94 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
#pragma GCC visibility push(default)
# 94 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"





typedef struct _cpuset cpuset_t;
# 124 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
int _sched_getaffinity(__pid_t, lwpid_t, size_t, cpuset_t *);
int _sched_setaffinity(__pid_t, lwpid_t, size_t, const cpuset_t *);
int _sched_getparam(__pid_t, lwpid_t, int *, struct sched_param *);
int _sched_setparam(__pid_t, lwpid_t, int, const struct sched_param *);
int _sched_protect(int);

# 129 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
#pragma GCC visibility pop
# 129 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"

# 148 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
struct kmutex;
# 158 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
struct schedstate_percpu {
 struct kmutex *spc_mutex;
 struct kmutex *spc_lwplock;
 struct lwp *spc_migrating;
 struct cpu_info *spc_nextpkg;
 psetid_t spc_psid;
 time_t spc_lastmod;
 volatile int spc_flags;
 u_int spc_schedticks;
 uint64_t spc_cp_time[5];
 int spc_ticks;
 int spc_pscnt;
 int spc_psdiv;
 int spc_nextskim;

 volatile pri_t spc_curpriority;
 pri_t spc_maxpriority;
 u_int spc_count;
 u_int spc_mcount;
 uint32_t spc_bitmap[8];
 struct { struct lwp *tqh_first; struct lwp * *tqh_last; } *spc_queue;
};
# 213 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
extern int schedhz;
extern u_int sched_rrticks;
extern u_int sched_pstats_ticks;

struct proc;
struct cpu_info;






void runq_init(void);
void synch_init(void);
void sched_init(void);
void sched_rqinit(void);
void sched_cpuattach(struct cpu_info *);


void sched_tick(struct cpu_info *);
void schedclock(struct lwp *);
void sched_schedclock(struct lwp *);
void sched_pstats(void);
void sched_lwp_stats(struct lwp *);
void sched_pstats_hook(struct lwp *, int);



# 240 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 3 4
_Bool 
# 240 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
     sched_curcpu_runnable_p(void);
void sched_dequeue(struct lwp *);
void sched_enqueue(struct lwp *);
void sched_preempted(struct lwp *);
void sched_resched_cpu(struct cpu_info *, pri_t, 
# 244 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 3 4
                                                 _Bool
# 244 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
                                                     );
void sched_resched_lwp(struct lwp *, 
# 245 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 3 4
                                     _Bool
# 245 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
                                         );
struct lwp * sched_nextlwp(void);
void sched_oncpu(struct lwp *);
void sched_newts(struct lwp *);
void sched_vforkexec(struct lwp *, 
# 249 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 3 4
                                   _Bool
# 249 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
                                       );


void sched_nice(struct proc *, int);


void sched_proc_fork(struct proc *, struct proc *);
void sched_proc_exit(struct proc *, struct proc *);
void sched_lwp_fork(struct lwp *, struct lwp *);
void sched_lwp_collect(struct lwp *);

void sched_slept(struct lwp *);
void sched_wakeup(struct lwp *);

void setrunnable(struct lwp *);
void sched_setrunnable(struct lwp *);

struct cpu_info *sched_takecpu(struct lwp *);
void sched_print_runqueue(void (*pr)(const char *, ...)
    __attribute__((__format__ (__printf__, 1, 2))));



# 271 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 3 4
_Bool 
# 271 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
     kpreempt(uintptr_t);
void preempt(void);

# 273 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h" 3 4
_Bool 
# 273 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/sched.h"
     preempt_needed(void);
void preempt_point(void);
void yield(void);
void mi_switch(struct lwp *);
void updatertime(lwp_t *, const struct bintime *);
void sched_idle(void);
void suspendsched(void);

int do_sched_setparam(__pid_t, lwpid_t, int, const struct sched_param *);
int do_sched_getparam(__pid_t, lwpid_t, int *, struct sched_param *);
# 41 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 2

void kcpuset_sysinit(void);

void kcpuset_create(kcpuset_t **, 
# 44 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 3 4
                                  _Bool
# 44 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
                                      );
void kcpuset_clone(kcpuset_t **, const kcpuset_t *);
void kcpuset_destroy(kcpuset_t *);
void kcpuset_copy(kcpuset_t *, const kcpuset_t *);

void kcpuset_use(kcpuset_t *);
void kcpuset_unuse(kcpuset_t *, kcpuset_t **);

int kcpuset_copyin(const cpuset_t *, kcpuset_t *, size_t);
int kcpuset_copyout(kcpuset_t *, cpuset_t *, size_t);

void kcpuset_zero(kcpuset_t *);
void kcpuset_fill(kcpuset_t *);
void kcpuset_set(kcpuset_t *, cpuid_t);
void kcpuset_clear(kcpuset_t *, cpuid_t);


# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 3 4
_Bool 
# 60 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
     kcpuset_isset(const kcpuset_t *, cpuid_t);

# 61 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 3 4
_Bool 
# 61 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
     kcpuset_isotherset(const kcpuset_t *, cpuid_t);

# 62 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 3 4
_Bool 
# 62 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
     kcpuset_iszero(const kcpuset_t *);

# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 3 4
_Bool 
# 63 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
     kcpuset_intersecting_p(const kcpuset_t *, const kcpuset_t *);

# 64 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h" 3 4
_Bool 
# 64 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/kcpuset.h"
     kcpuset_match(const kcpuset_t *, const kcpuset_t *);
void kcpuset_intersect(kcpuset_t *, const kcpuset_t *);
void kcpuset_merge(kcpuset_t *, const kcpuset_t *);
void kcpuset_remove(kcpuset_t *, const kcpuset_t *);
int kcpuset_countset(const kcpuset_t *);

cpuid_t kcpuset_ffs(const kcpuset_t *);
cpuid_t kcpuset_ffs_intersecting(const kcpuset_t *, const kcpuset_t *);

void kcpuset_atomic_set(kcpuset_t *, cpuid_t);
void kcpuset_atomic_clear(kcpuset_t *, cpuid_t);

void kcpuset_atomicly_zero(kcpuset_t *);
void kcpuset_atomicly_intersect(kcpuset_t *, const kcpuset_t *);
void kcpuset_atomicly_merge(kcpuset_t *, const kcpuset_t *);
void kcpuset_atomicly_remove(kcpuset_t *, const kcpuset_t *);

void kcpuset_export_u32(const kcpuset_t *, uint32_t *, size_t);
# 43 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h" 2


# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/resource.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/resource.h"
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/featuretest.h" 1
# 38 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/resource.h" 2
# 57 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/resource.h"
struct rusage {
 struct timeval ru_utime;
 struct timeval ru_stime;
 long ru_maxrss;

 long ru_ixrss;
 long ru_idrss;
 long ru_isrss;
 long ru_minflt;
 long ru_majflt;
 long ru_nswap;
 long ru_inblock;
 long ru_oublock;
 long ru_msgsnd;
 long ru_msgrcv;
 long ru_nsignals;
 long ru_nvcsw;
 long ru_nivcsw;

};


struct wrusage {
        struct rusage wru_self;
 struct rusage wru_children;
};
# 127 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/resource.h"
struct orlimit {
 int32_t rlim_cur;
 int32_t rlim_max;
};


struct rlimit {
 rlim_t rlim_cur;
 rlim_t rlim_max;
};



struct loadavg {
 fixpt_t ldavg[3];
 long fscale;
};



extern struct loadavg averunnable;
struct pcred;
int dosetrlimit(struct lwp *, struct proc *, int, struct rlimit *);
# 46 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h" 1
# 54 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
typedef struct ksiginfoq { struct ksiginfo *tqh_first; struct ksiginfo * *tqh_last; } ksiginfoq_t;




struct sigacts {
 struct sigact_sigdesc {
  struct sigaction sd_sigact;
  const void *sd_tramp;
  int sd_vers;
 } sa_sigdesc[64];

 int sa_refcnt;
 kmutex_t sa_mutex;
};




typedef struct sigpend {
 ksiginfoq_t sp_info;
 sigset_t sp_set;
} sigpend_t;




struct sigctx {
 struct _ksiginfo ps_info;
 int ps_lwp;
 
# 84 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h" 3 4
_Bool 
# 84 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
       ps_faked;
 void *ps_sigcode;
 sigset_t ps_sigignore;
 sigset_t ps_sigcatch;
 sigset_t ps_sigpass;
};
# 103 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
static __inline void
sigaction_copy(struct sigaction *dst, const struct sigaction *src)
{
 __builtin_memset(dst, 0, sizeof(*dst));
 dst->_sa_u._sa_handler = src->_sa_u._sa_handler;
 __builtin_memcpy(&dst->sa_mask, &src->sa_mask, sizeof(dst->sa_mask));
 dst->sa_flags = src->sa_flags;
}
# 132 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
extern sigset_t contsigmask, stopsigmask, sigcantmask;

struct vnode;
struct coredump_iostate;




int coredump_netbsd(struct lwp *, struct coredump_iostate *);
int coredump_netbsd32(struct lwp *, struct coredump_iostate *);
int real_coredump_netbsd(struct lwp *, struct coredump_iostate *);
void execsigs(struct proc *);
int issignal(struct lwp *);
void pgsignal(struct pgrp *, int, int);
void kpgsignal(struct pgrp *, struct ksiginfo *, void *, int);
void postsig(int);
void psignal(struct proc *, int);
void kpsignal(struct proc *, struct ksiginfo *, void *);
void child_psignal(struct proc *, int);
void siginit(struct proc *);
void trapsignal(struct lwp *, struct ksiginfo *);
void sigexit(struct lwp *, int) __attribute__((__noreturn__));
void killproc(struct proc *, const char *);
void setsigvec(struct proc *, int, struct sigaction *);
int killpg1(struct lwp *, struct ksiginfo *, int, int);
void proc_unstop(struct proc *p);
void eventswitch(int, int, int);
void eventswitchchild(struct proc *, int, int);

int sigaction1(struct lwp *, int, const struct sigaction *,
     struct sigaction *, const void *, int);
int sigprocmask1(struct lwp *, int, const sigset_t *, sigset_t *);
void sigpending1(struct lwp *, sigset_t *);
void sigsuspendsetup(struct lwp *, const sigset_t *);
void sigsuspendteardown(struct lwp *);
int sigsuspend1(struct lwp *, const sigset_t *);
int sigaltstack1(struct lwp *, const stack_t *, stack_t *);
int sigismasked(struct lwp *, int);

int sigget(sigpend_t *, ksiginfo_t *, int, const sigset_t *);
void sigclear(sigpend_t *, const sigset_t *, ksiginfoq_t *);
void sigclearall(struct proc *, const sigset_t *, ksiginfoq_t *);

int kpsignal2(struct proc *, ksiginfo_t *);

void signal_init(void);

struct sigacts *sigactsinit(struct proc *, int);
void sigactsunshare(struct proc *);
void sigactsfree(struct sigacts *);

void kpsendsig(struct lwp *, const struct ksiginfo *, const sigset_t *);
void sendsig_reset(struct lwp *, int);
void sendsig(const struct ksiginfo *, const sigset_t *);

ksiginfo_t *ksiginfo_alloc(struct proc *, ksiginfo_t *, int);
void ksiginfo_free(ksiginfo_t *);
void ksiginfo_queue_drain0(ksiginfoq_t *);

struct sys_____sigtimedwait50_args;
int sigtimedwait1(struct lwp *, const struct sys_____sigtimedwait50_args *,
    register_t *, copyin_t, copyout_t, copyin_t, copyout_t);

void signotify(struct lwp *);
int sigispending(struct lwp *, int);




void sendsig_sigcontext(const struct ksiginfo *, const sigset_t *);
void sendsig_siginfo(const struct ksiginfo *, const sigset_t *);

extern struct pool ksiginfo_pool;






static __inline int
firstsig(const sigset_t *ss)
{
 int sig;

 sig = __builtin_ffs(ss->__bits[0]);
 if (sig != 0)
  return (sig);

 sig = __builtin_ffs(ss->__bits[1]);
 if (sig != 0)
  return (sig + 32);
# 234 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
 return (0);
}

static __inline void
ksiginfo_queue_init(ksiginfoq_t *kq)
{
 do { (kq)->tqh_first = (
# 240 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h" 3 4
((void *)0)
# 240 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
); (kq)->tqh_last = &(kq)->tqh_first; } while (0);
}

static __inline void
ksiginfo_queue_drain(ksiginfoq_t *kq)
{
 if (!(((kq)->tqh_first) == (
# 246 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h" 3 4
     ((void *)0)
# 246 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
     )))
  ksiginfo_queue_drain0(kq);
}
# 322 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/signalvar.h"
extern const int sigprop[64];
# 48 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h" 2
# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/specificdata.h" 1
# 37 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/specificdata.h"
typedef unsigned int specificdata_key_t;
typedef void (*specificdata_dtor_t)(void *);
typedef struct specificdata_domain *specificdata_domain_t;
typedef struct specificdata_container *specificdata_container_t;

typedef struct {
 specificdata_container_t specdataref_container;
 kmutex_t specdataref_lock;
} specificdata_reference;

specificdata_domain_t specificdata_domain_create(void);
void specificdata_domain_delete(specificdata_domain_t);

int specificdata_key_create(specificdata_domain_t,
    specificdata_key_t *, specificdata_dtor_t);
void specificdata_key_delete(specificdata_domain_t, specificdata_key_t);

int specificdata_init(specificdata_domain_t, specificdata_reference *);
void specificdata_fini(specificdata_domain_t, specificdata_reference *);

void * specificdata_getspecific(specificdata_domain_t,
     specificdata_reference *, specificdata_key_t);
void * specificdata_getspecific_unlocked(specificdata_domain_t,
       specificdata_reference *,
       specificdata_key_t);
void specificdata_setspecific(specificdata_domain_t,
     specificdata_reference *, specificdata_key_t,
     void *);
int specificdata_setspecific_nowait(specificdata_domain_t,
     specificdata_reference *,
     specificdata_key_t, void *);
# 49 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h" 2

# 1 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/wchan.h" 1
# 35 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/wchan.h"
typedef volatile const void *wchan_t;
# 51 "/mnt/e/lh/lsr/Ainux/custom_kernel/external/netbsd-src/sys/sys/lwp.h" 2


struct lwp;

static __inline struct cpu_info *lwp_getcpu(struct lwp *);