#ifndef _STDIO_H
#define _STDIO_H

/* Nano-C Standard I/O Library */

/* printf is a built-in keyword in Nano-C, so we don't need to declare it. */
/* print is also built-in. */

void puts(char *s) {
    print(s);
    print("\n");
}

void putchar(int c) {
    char s[2];
    s[0] = c;
    s[1] = 0;
    print(s);
}

/* Input functions */
/* input() is a built-in keyword that returns an int (or char?) */
/* Let's wrap it */

int getchar() {
    return input();
}

#endif
