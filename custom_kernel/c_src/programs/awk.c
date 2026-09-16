/* Minimal awk for Ainux — compiled with -nostdlib, raw syscalls via inline asm
 * Features:
 * - Simple field extraction: {print $1} or {print $0}
 * - Basic Regex pattern matching: /regex/ {print $N}
 * - Regex ops: ^, $, ., *, +, ?
 */
typedef unsigned long long u64;
typedef long long i64;
typedef unsigned int u32;
typedef int i32;

/* ── Raw syscall wrappers ──────────────────────────────────────────────── */

static i64 sys0(u64 id) {
    i64 ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(id)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static i64 sys3(u64 id, u64 a1, u64 a2, u64 a3) {
    i64 ret;
    register u64 r10 __asm__("r10") = 0;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(id), "D"(a1), "S"(a2), "d"(a3), "r"(r10)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static i64 ainux_write(u64 fd, const char *buf, u64 len) {
    return sys3(1, fd, (u64)(unsigned long)buf, len);
}

static i64 ainux_read(u64 fd, char *buf, u64 len) {
    return sys3(0, fd, (u64)(unsigned long)buf, len);
}

static void ainux_exit(i32 code) {
    sys3(60, (u64)code, 0, 0);
    __builtin_unreachable();
}

/* ── str utilities ─────────────────────────────────────────────────────── */

static unsigned long ainux_strlen(const char *s) {
    unsigned long n = 0;
    while (s[n]) n++;
    return n;
}

static void puts_fd(u64 fd, const char *s) {
    ainux_write(fd, s, ainux_strlen(s));
}

/* ── Minimal Regex Engine ──────────────────────────────────────────────── */
/* Supports: ^, $, ., *, +, ? */

static int match_here(const char *re, const char *text);

static int match_star(int c, const char *re, const char *text) {
    do {
        if (match_here(re, text)) return 1;
    } while (*text != '\0' && (*text++ == c || c == '.'));
    return 0;
}

static int match_plus(int c, const char *re, const char *text) {
    if (*text != '\0' && (*text == c || c == '.')) {
        text++;
        return match_star(c, re, text);
    }
    return 0;
}

static int match_opt(int c, const char *re, const char *text) {
    if (match_here(re, text)) return 1;
    if (*text != '\0' && (*text == c || c == '.')) {
        return match_here(re, text + 1);
    }
    return 0;
}

static int match_here(const char *re, const char *text) {
    if (re[0] == '\0') return 1;
    if (re[1] == '*') return match_star(re[0], re + 2, text);
    if (re[1] == '+') return match_plus(re[0], re + 2, text);
    if (re[1] == '?') return match_opt(re[0], re + 2, text);
    if (re[0] == '$' && re[1] == '\0') return *text == '\0';
    if (*text != '\0' && (re[0] == '.' || re[0] == *text)) {
        return match_here(re + 1, text + 1);
    }
    return 0;
}

static int match_regex(const char *re, const char *text) {
    if (re[0] == '^') return match_here(re + 1, text);
    do {
        if (match_here(re, text)) return 1;
    } while (*text++ != '\0');
    return 0;
}

/* ── Awk Parser and Execution ──────────────────────────────────────────── */

typedef struct {
    char regex[128];
    int target_field; /* 0 means print whole line, >0 means specific field */
    int has_regex;
} AwkCmd;

static int parse_cmd(const char *cmd, AwkCmd *parsed) {
    parsed->target_field = -1;
    parsed->has_regex = 0;
    parsed->regex[0] = '\0';

    int i = 0;
    
    // Check for optional regex pattern: /regex/
    while (cmd[i] == ' ' || cmd[i] == '\t') i++;
    if (cmd[i] == '/') {
        i++;
        int r = 0;
        while (cmd[i] != '\0' && cmd[i] != '/') {
            if (r < sizeof(parsed->regex) - 1) {
                parsed->regex[r++] = cmd[i];
            }
            i++;
        }
        parsed->regex[r] = '\0';
        parsed->has_regex = 1;
        if (cmd[i] == '/') i++;
    }

    // Check for action block: {print $N}
    while (cmd[i] == ' ' || cmd[i] == '\t') i++;
    if (cmd[i] == '{') {
        for (; cmd[i] != '\0'; i++) {
            if (cmd[i] == '$' && cmd[i+1] >= '0' && cmd[i+1] <= '9') {
                parsed->target_field = cmd[i+1] - '0';
                return 1; // Successfully parsed action
            }
        }
    }
    return 0; // Failed to parse
}

static void process_line(char *line, const AwkCmd *cmd) {
    // If regex is provided, skip line if no match
    if (cmd->has_regex) {
        if (!match_regex(cmd->regex, line)) {
            return;
        }
    }

    if (cmd->target_field == 0) {
        // Print whole line
        puts_fd(1, line);
        ainux_write(1, "\n", 1);
        return;
    }

    // Split and extract Nth field
    int current_field = 1;
    char *field_start = line;
    int in_space = 0;

    for (char *c = line; ; c++) {
        int end = (*c == '\0');
        if (*c == ' ' || *c == '\t' || end) {
            if (!in_space && c != line) {
                /* End of a field */
                char saved = *c;
                *c = '\0';
                if (current_field == cmd->target_field) {
                    puts_fd(1, field_start);
                    ainux_write(1, "\n", 1);
                    return;
                }
                *c = saved;
                current_field++;
                in_space = 1;
            }
            if (end) break;
        } else {
            if (in_space) {
                field_start = c;
                in_space = 0;
            }
        }
    }
}

void _start(void) {
    puts_fd(1, "awk: use via shell pipe, e.g.: ls | awk '{print $1}'\n");
    ainux_exit(0);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        puts_fd(2, "Usage: awk '[/pattern/] {print $N}'\n");
        ainux_exit(1);
    }

    AwkCmd cmd;
    if (!parse_cmd(argv[1], &cmd)) {
        puts_fd(2, "awk: unsupported command syntax. Use '[/pattern/] {print $N}'\n");
        ainux_exit(1);
    }

    char buf[2048];
    char line[512];
    int line_len = 0;
    i64 n;

    while ((n = ainux_read(0, buf, sizeof(buf))) > 0) {
        for (i64 i = 0; i < n; i++) {
            char c = buf[i];
            if (c == '\n' || line_len >= (int)sizeof(line) - 1) {
                line[line_len] = '\0';
                process_line(line, &cmd);
                line_len = 0;
            } else {
                line[line_len++] = c;
            }
        }
    }
    
    if (line_len > 0) {
        line[line_len] = '\0';
        process_line(line, &cmd);
    }

    ainux_exit(0);
}
