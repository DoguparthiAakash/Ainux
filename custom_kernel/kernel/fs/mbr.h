#ifndef MBR_H
#define MBR_H

void mbr_write_default(void);
void mbr_print_map(void);
int mbr_new_partition(uint32_t size_mb);

#endif
