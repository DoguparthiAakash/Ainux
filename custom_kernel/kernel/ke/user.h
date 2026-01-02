#ifndef USER_H
#define USER_H

/* User API */
void user_init(void);
int user_check(const char *username, const char *password);
int user_add(const char *username, const char *password, const char *email, const char *mobile);
int user_login_loop(void);

const char* user_get_current(void);
void user_set_current(const char *username);
int user_is_authenticated(void);
void user_set_authenticated(int status);

int user_recover_check(const char *email, const char *mobile, char *out_user);
int user_reset_pass(const char *username, const char *new_pass);

/* Blocking login loop. Returns 1 when authenticated. */
/* If no users exist, it may prompt to create a root user. */
int user_login_loop(void);

void user_get_input(char *buf, int max);
void user_get_input_masked(char *buf, int max);

const char* user_get_current(void);

#endif
