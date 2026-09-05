#include "gpt.h"
#include "../drivers/ata.h"
#include "../libc/stdio.h"
#include "../libc/string.h"

extern void *kmalloc(uint64_t size);
extern void kfree(void *ptr);

#define GPT_SIGNATURE 0x5452415020494645ULL  /* "EFI PART" */

int gpt_read_header(struct gpt_header *header) {
    uint8_t *buffer = (uint8_t *)kmalloc(512);
    if (!buffer) return -1;
    
    /* GPT header is at LBA 1 */
    ata_read_sectors(1, 1, buffer);
    memcpy(header, buffer, sizeof(struct gpt_header));
    
    kfree(buffer);
    
    /* Validate signature */
    if (header->signature != GPT_SIGNATURE) {
        return -1;
    }
    
    return 0;
}

static int guid_matches(const uint8_t *guid1, const uint8_t *guid2) {
    for (int i = 0; i < 16; i++) {
        if (guid1[i] != guid2[i]) return 0;
    }
    return 1;
}

int gpt_find_fat32_partition(uint32_t *lba_start) {
    struct gpt_header header;
    
    if (gpt_read_header(&header) != 0) {
        return -1;  /* Not a GPT disk */
    }
    
    printf("[GPT] Valid GPT header found\n");
    printf("[GPT] Partition entries at LBA %llu, count: %u\n", 
           header.partition_entries_lba, header.num_partition_entries);
    
    /* Read partition entries */
    uint8_t *entries_buffer = (uint8_t *)kmalloc(512 * 4);  /* Read 4 sectors worth */
    if (!entries_buffer) return -1;
    
    ata_read_sectors(header.partition_entries_lba, 4, entries_buffer);
    
    /* EFI System Partition and Basic Data GUIDs */
    const uint8_t efi_system_guid[] = GPT_TYPE_EFI_SYSTEM;
    const uint8_t basic_data_guid[] = GPT_TYPE_BASIC_DATA;
    
    struct gpt_partition_entry *entries = (struct gpt_partition_entry *)entries_buffer;
    
    /* Scan for FAT32-compatible partitions */
    for (uint32_t i = 0; i < header.num_partition_entries && i < 32; i++) {
        struct gpt_partition_entry *entry = &entries[i];
        
        /* Check if entry is valid (non-zero type GUID) */
        int is_zero = 1;
        for (int j = 0; j < 16; j++) {
            if (entry->type_guid[j] != 0) {
                is_zero = 0;
                break;
            }
        }
        if (is_zero) continue;
        
        /* Check for EFI System or Basic Data partitions */
        if (guid_matches(entry->type_guid, efi_system_guid) ||
            guid_matches(entry->type_guid, basic_data_guid)) {
            
            *lba_start = (uint32_t)entry->first_lba;
            
            printf("[GPT] Found partition %u at LBA %u\n", i, *lba_start);
            kfree(entries_buffer);
            return 0;
        }
    }
    
    kfree(entries_buffer);
    printf("[GPT] No suitable partition found\n");
    return -1;
}
