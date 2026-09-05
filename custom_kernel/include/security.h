#ifndef SECURITY_H
#define SECURITY_H

#include <stdint.h>

/* Security Hardening Defaults */
#define USER_SPACE_LIMIT 0x00007FFFFFFFFFFF

/* Prototypes */
int validate_user_pointer(const void *ptr, uint64_t size);
int validate_user_string(const char *ptr, uint64_t max_len);

/* Copy Helpers that perform validation */
int copy_from_user(void *dest, const void *src, uint64_t size);
int copy_to_user(void *dest, const void *src, uint64_t size);

#endif
