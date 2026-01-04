#ifndef _MATH_H
#define _MATH_H

/* Nano-C Math Library */

int abs(int x) {
    if (x < 0) return -x;
    return x;
}

int max(int a, int b) {
    if (a > b) return a;
    return b;
}

int min(int a, int b) {
    if (a < b) return a;
    return b;
}

int pow(int base, int exp) {
    int res = 1;
    while (exp > 0) {
        res = res * base;
        exp--;
    }
    return res;
}

int sqrt(int n) {
    if (n < 0) return -1;
    if (n == 0) return 0;
    int x = n;
    int y = 1;
    while (x > y) {
        x = (x + y) / 2;
        y = n / x;
    }
    return x;
}

#endif
