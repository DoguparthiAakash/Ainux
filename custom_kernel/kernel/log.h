#ifndef _LOG_H
#define _LOG_H

void klog_init(void);
void klog_write(const char *msg);
void klog_dump(void);
void kprint(const char *msg);
void kprint_color(int color, const char *msg);

#define KLOG_COLOR_RESET   0
#define KLOG_COLOR_RED     1
#define KLOG_COLOR_GREEN   2
#define KLOG_COLOR_YELLOW  3
#define KLOG_COLOR_BLUE    4
#define KLOG_COLOR_CYAN    5

#endif
