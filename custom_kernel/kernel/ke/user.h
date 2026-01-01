#ifndef USER_H
#define USER_H

/* Initialize user subsystem (ensure DB exists) */
void user_init(void);

/* Check credentials. Returns 1 on success, 0 on fail. */
int user_check(const char *username, const char *password);

/* Add new user. Returns 0 on success. */
int user_add(const char *username, const char *password);

/* Blocking login loop. Returns 1 when authenticated. */
/* If no users exist, it may prompt to create a root user. */
int user_login_loop(void);

void user_get_input(char *buf, int max);
void user_get_input_masked(char *buf, int max);

const char* user_get_current(void);

#endif
