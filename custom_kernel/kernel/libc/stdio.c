#include "stdio.h"
#include "string.h"
#include "stdlib.h"
#include "../fs/initrd.h"

extern void kprint(const char *msg);

FILE *stdin = NULL;
FILE *stdout = NULL;
FILE *stderr = NULL;

/* Simple vsnprintf implementation */
/* Supports %d, %s, %c, %x, %p */
static char *print_num(char *str, long num, int base, int width, int pad_zero) {
    if (num == 0) {
        *str++ = '0';
        return str;
    }
    int neg = 0;
    if (num < 0 && base == 10) {
        neg = 1;
        num = -num;
    }
    unsigned long u = (unsigned long)num;
    char buf[32];
    int i = 0;
    while (u > 0) {
        int d = u % base;
        buf[i++] = (d < 10) ? ('0' + d) : ('a' + d - 10);
        u /= base;
    }
    if (neg) buf[i++] = '-';
    
    while (i < width) {
        buf[i++] = pad_zero ? '0' : ' ';
    }
    
    while (i > 0) {
        *str++ = buf[--i];
    }
    return str;
}

int vsnprintf(char *str, size_t size, const char *format, va_list ap) {
    char *start = str;
    char *end = str + size - 1;
    
    while (*format && str < end) {
        if (*format != '%') {
            *str++ = *format++;
            continue;
        }
        format++;
        
        int width = 0;
        int pad_zero = 0;
        if (*format == '0') {
            pad_zero = 1;
            format++;
        }
        while (*format >= '0' && *format <= '9') {
            width = width * 10 + (*format - '0');
            format++;
        }
        
        if (*format == 's') {
            char *s = va_arg(ap, char *);
            if (!s) s = "(null)";
            while (*s && str < end) *str++ = *s++;
        } else if (*format == 'd' || *format == 'i') {
            int d = va_arg(ap, int);
            str = print_num(str, d, 10, width, pad_zero);
        } else if (*format == 'x') {
            int x = va_arg(ap, int);
            str = print_num(str, x, 16, width, pad_zero);
        } else if (*format == 'c') {
            char c = (char)va_arg(ap, int);
            *str++ = c;
        } else if (*format == '%') {
            *str++ = '%';
        }
        format++;
    }
    *str = '\0';
    return str - start;
}

int vsprintf(char *str, const char *format, va_list ap) {
    return vsnprintf(str, 0xFFFFFFFF, format, ap);
}

int sprintf(char *str, const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    int ret = vsprintf(str, format, ap);
    va_end(ap);
    return ret;
}

int snprintf(char *str, size_t size, const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    int ret = vsnprintf(str, size, format, ap);
    va_end(ap);
    return ret;
}

int printf(const char *format, ...) {
    char buf[1024];
    va_list ap;
    va_start(ap, format);
    int ret = vsnprintf(buf, sizeof(buf), format, ap);
    va_end(ap);
    kprint(buf);
    return ret;
}

int puts(const char *s) {
    kprint(s);
    kprint("\n");
    return 0;
}

int putchar(int c) {
    char s[2] = { (char)c, 0 };
    kprint(s);
    return c;
}

/* FILE I/O */

FILE *fopen(const char *pathname, const char *mode) {
    // Read-only support from InitRD for now
    struct initrd_file *f = initrd_find_file(pathname);
    if (!f) return NULL;
    
    FILE *stream = (FILE *)malloc(sizeof(FILE));
    if (!stream) return NULL;
    
    stream->data = (char *)f->data;
    stream->size = (size_t)f->size;
    stream->pos = 0;
    stream->mode = 0; // Read
    
    return stream;
}

size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream) {
    if (!stream || !ptr) return 0;
    
    size_t bytes_to_read = size * nmemb;
    size_t remaining = stream->size - stream->pos;
    
    if (bytes_to_read > remaining) bytes_to_read = remaining;
    
    memcpy(ptr, stream->data + stream->pos, bytes_to_read);
    stream->pos += bytes_to_read;
    
    return bytes_to_read / size;
}

size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream) {
    // Write not supported directly to file FILE * yet (initrd is tricky)
    (void)ptr; (void)size; (void)nmemb; (void)stream;
    return 0;
}

int fseek(FILE *stream, long offset, int whence) {
    if (!stream) return -1;
    
    size_t new_pos = 0;
    if (whence == SEEK_SET) new_pos = offset;
    else if (whence == SEEK_CUR) new_pos = stream->pos + offset;
    else if (whence == SEEK_END) new_pos = stream->size + offset;
    
    if (new_pos > stream->size) new_pos = stream->size;
    
    stream->pos = new_pos;
    return 0;
}

long ftell(FILE *stream) {
    if (!stream) return -1;
    return stream->pos;
}

int fclose(FILE *stream) {
    if (stream) free(stream);
    return 0;
}

int fflush(FILE *stream) {
    (void)stream;
    return 0;
}

int fgetc(FILE *stream) {
    if (!stream || stream->pos >= stream->size) return EOF;
    return (unsigned char)stream->data[stream->pos++];
}

char *fgets(char *s, int size, FILE *stream) {
    if (!stream || size <= 0) return NULL;
    
    int i = 0;
    int c;
    while (i < size - 1 && (c = fgetc(stream)) != EOF) {
        s[i++] = c;
        if (c == '\n') break;
    }
    if (i == 0) return NULL;
    s[i] = '\0';
    return s;
}
