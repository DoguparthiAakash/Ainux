#ifndef _LOG_H
#define _LOG_H

void klog_init(void);
void klog_write(const char *msg);
void klog_dump(void);
void kprint(const char *msg);

#endif
