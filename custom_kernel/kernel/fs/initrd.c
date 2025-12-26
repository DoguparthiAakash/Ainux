#include "initrd.h"
#include <limine.h>
#include "../mm/heap.h"

extern void kprint(const char *msg);

/* Limine module request */
__attribute__((used, section(".requests")))
static volatile struct limine_module_request module_request = {
    .id = LIMINE_MODULE_REQUEST,
    .revision = 0
};

static uint8_t *initrd_start = NULL;
static uint64_t initrd_size = 0;
static uint64_t initrd_capacity = 0;
static int is_writable = 0;

static uint64_t octal_to_int(const char *str, size_t len) {
    uint64_t result = 0;
    /* Strip leading spaces/nulls if any (standard tar doesn't use them for size usually but safe to skip) */
    /* Implementation assumes standard format */
    for (size_t i = 0; i < len && str[i] >= '0' && str[i] <= '7'; i++) {
        result = result * 8 + (str[i] - '0');
    }
    return result;
}

static void int_to_octal(uint64_t num, char *str, size_t len) {
    for (size_t i=0; i<len-1; i++) str[i] = '0';
    str[len-1] = '\0';
    if (num == 0) return;
    size_t pos = len - 2;
    while (num > 0 && pos < len) { 
        str[pos] = '0' + (num % 8);
        num /= 8;
        if (pos == 0) break;
        pos--;
    }
}

static int str_cmp(const char *s1, const char *s2) {
    while (*s1 && *s2 && *s1 == *s2) {
        s1++;
        s2++;
    }
    return *s1 - *s2;
}

static void str_cpy(char *dest, const char *src, size_t max) {
    size_t i = 0;
    while (i < max - 1 && src[i]) {
        dest[i] = src[i];
        i++;
    }
    dest[i] = '\0';
}

static int str_starts_with(const char *str, const char *prefix) {
    while (*prefix) {
        if (*str != *prefix) return 0;
        str++;
        prefix++;
    }
    return 1;
}

static size_t str_len(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    return len;
}

static void *mem_cpy(void *dest, const void *src, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
    return dest;
}

static void mem_set(void *dest, int val, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    while (n--) *d++ = (uint8_t)val;
}

/* Helper to strip ./ prefix */
static const char *normalize_name(const char *name) {
    if (name[0] == '.' && name[1] == '/') return name + 2;
    return name;
}

int initrd_init(void) {
    if (module_request.response == NULL || module_request.response->module_count == 0) {
        kprint("[InitRD] No modules found!\n");
        return -1;
    }
    struct limine_file *module = module_request.response->modules[0];
    
    initrd_capacity = 1024 * 1024;
    initrd_start = (uint8_t *)kmalloc(initrd_capacity);
    
    if (!initrd_start) {
        kprint("[InitRD] Failed to allocate RAM buffer!\n");
        initrd_start = (uint8_t *)module->address;
        initrd_size = module->size;
        initrd_capacity = module->size;
        is_writable = 0;
        kprint("[InitRD] Fallback to Read-Only mode\n");
    } else {
        uint64_t copy_size = (module->size > initrd_capacity) ? initrd_capacity : module->size;
        mem_cpy(initrd_start, module->address, copy_size);
        
        /* Trim EOF zero blocks to determine real logical size */
        /* Scan headers until we find end */
        uint8_t *ptr = initrd_start;
        uint64_t real_size = 0;
        
        while (ptr < initrd_start + copy_size) {
            struct tar_header *header = (struct tar_header *)ptr;
            /* Check if zero block */
            if (header->filename[0] == '\0') {
                /* End of data found */
                break;
            }
            
            /* Basic validity check */
            if (header->magic[0] != 'u') {
                /* Not ustar, assume end or garbage */
                break;
            }

            uint64_t size = octal_to_int(header->size, 11);
            uint64_t block_size = 512 + ((size + 511) / 512) * 512;
            
            ptr += block_size;
            real_size += block_size;
        }
        
        initrd_size = real_size;
        is_writable = 1;
        
        /* Ensure we have EOF blocks after our trimmed size in the buffer */
        if (initrd_size + 1024 <= initrd_capacity) {
            mem_set(initrd_start + initrd_size, 0, 1024);
        }
        
        kprint("[InitRD] Loaded and Trimmed\n");
    }
    return 0;
}

static void update_checksum(struct tar_header *header) {
    uint32_t sum = 0;
    uint8_t *p = (uint8_t *)header;
    mem_set(header->checksum, ' ', 8);
    for (int i = 0; i < 512; i++) sum += p[i];
    int_to_octal(sum, header->checksum, 7); 
    header->checksum[6] = '\0'; 
    header->checksum[7] = ' '; 
}

int initrd_create_file(const char *name, const char *data, uint64_t len) {
    if (!is_writable) return -1;
    
    uint64_t needed = 512 + ((len + 511) / 512) * 512;
    if (initrd_size + needed > initrd_capacity) return -2;
    
    struct tar_header *header = (struct tar_header *)(initrd_start + initrd_size);
    mem_set(header, 0, 512);
    
    if (name[0] == '/') name++;
    if (name[0] == '.' && name[1] == '/') name += 2;
    
    str_cpy(header->filename, name, 100);
    str_cpy(header->mode, "000644", 8);
    str_cpy(header->uid, "000000", 8);
    str_cpy(header->gid, "000000", 8);
    int_to_octal(len, header->size, 12);
    int_to_octal(0, header->mtime, 12);
    header->typeflag = '0'; 
    str_cpy(header->magic, "ustar ", 6);
    str_cpy(header->version, " \0", 2);
    
    update_checksum(header);
    
    void *data_dest = (void *)((uint8_t *)header + 512);
    mem_set(data_dest, 0, ((len + 511) / 512) * 512);
    if (data && len > 0) mem_cpy(data_dest, data, len);
    
    initrd_size += needed;
    
    if (initrd_size + 1024 <= initrd_capacity) mem_set(initrd_start + initrd_size, 0, 1024);
    
    return 0;
}

int initrd_create_dir(const char *name) {
    if (!is_writable) return -1;
    
    uint64_t needed = 512;
    if (initrd_size + needed > initrd_capacity) return -2;
    
    struct tar_header *header = (struct tar_header *)(initrd_start + initrd_size);
    mem_set(header, 0, 512);
    
    if (name[0] == '/') name++;
    if (name[0] == '.' && name[1] == '/') name += 2;
    
    str_cpy(header->filename, name, 100);
    size_t nlen = str_len(header->filename);
    if (nlen < 99 && header->filename[nlen-1] != '/') {
        header->filename[nlen] = '/';
        header->filename[nlen+1] = '\0';
    }
    
    str_cpy(header->mode, "000755", 8);
    str_cpy(header->uid, "000000", 8);
    str_cpy(header->gid, "000000", 8);
    int_to_octal(0, header->size, 12);
    int_to_octal(0, header->mtime, 12);
    header->typeflag = '5'; 
    str_cpy(header->magic, "ustar ", 6);
    str_cpy(header->version, " \0", 2);
    
    update_checksum(header);
    
    initrd_size += needed;
    
    if (initrd_size + 1024 <= initrd_capacity) mem_set(initrd_start + initrd_size, 0, 1024);
    
    return 0;
}

int initrd_delete_file(const char *name) {
    if (!initrd_start || !is_writable) return -1;
    
    uint8_t *ptr = initrd_start;
    int found = 0;
    while (ptr < initrd_start + initrd_size) {
        struct tar_header *header = (struct tar_header *)ptr;
        if (header->filename[0] == '\0') break;
        if (header->magic[0] != 'u') break; /* Integrity check */

        uint64_t size = octal_to_int(header->size, 11);
        
        int match = 0;
        const char *hname = normalize_name(header->filename);
        if (name[0] == '.' && name[1] == '/') name+=2; 
        if (name[0] == '/') name++;

        if (str_cmp(hname, name) == 0) match=1;
        
        if (match) {
            header->typeflag = 'X'; 
            found = 1;
            /* Continue searching to mark all matching versions as deleted. */
        }
        uint64_t total_size = 512 + ((size + 511) / 512) * 512;
        ptr += total_size;
    }
    return found ? 0 : -1;
}

void initrd_list_files(const char *path) {
    if (!initrd_start) {
        kprint("[InitRD] Not initialized!\n");
        return;
    }

    if (!path || path[0] == '\0') path = "/";
    if (path[0] == '.' && path[1] == '/') path += 2; /* skip ./ */
    
    kprint("[InitRD] Listing: ");
    kprint(path);
    kprint("\n");
    
    uint8_t *ptr = initrd_start;
    while (ptr < initrd_start + initrd_size) {
        struct tar_header *header = (struct tar_header *)ptr;
        if (header->filename[0] == '\0') break;
        
        /* Safety check */
        if (header->magic[0] != 'u') {
             /* Corrupt or unexpected end */
             break;
        }

        uint64_t size = octal_to_int(header->size, 11);
        
        /* Iterate to see if this file is deleted later? No, complex. 
           We just trust 'typeflag=X' means deleted. 
           But wait, if we have [FW1, FW2_deleted, FW3], we should show FW3.
           If we have [FW1, FW2, FW3], all are valid if simple append logic.
           The 'find_file' logic finds LAST one. 'list' logic usually shows all unique names.
           Let's scan ALL files, store unique names in a temp buffer? Too much memory.
           Simple Hack: Just list everything not marked X. User will see duplicates if they exist and are valid.
           But delete logic marks ALL as X. So only active files remain non-X.
           Wait, if I create a NEW file, it appends. The old ones are marked X by my new delete logic.
           So `list` just skipping X is sufficient!
        */
        
        if (header->typeflag != 'X') { 
            const char *name = normalize_name(header->filename);
            
            int match = 0;
            if (path[0] == '/' && path[1] == '\0') {
                /* Root: print only if no slash in name (excluding trailing slash) */
                int has_slash = 0;
                for (int i = 0; name[i]; i++) {
                    if (name[i] == '/' && name[i+1] != '\0') has_slash = 1;
                }
                if (!has_slash) match = 1;
            } else {
                 /* Directory: print if it starts with path and has only one extra slash level (optional) */
                 const char *p = (path[0] == '/') ? path + 1 : path;
                 if (str_starts_with(name, p)) match = 1;
            }
    
            if (match) {
                kprint("  - ");
                kprint(name);
                if (header->typeflag == '5') kprint("/");
                kprint("\n");
            }
        }

        uint64_t total_size = 512 + ((size + 511) / 512) * 512;
        ptr += total_size;
    }
}

int initrd_is_dir(const char *path) {
    if (!initrd_start) return 0;
    if (!path || (path[0] == '/' && path[1] == '\0')) return 1;

    /* Normalize */
    if (path[0] == '.') {
        if (path[1] == '/') path += 2;
        else if (path[1] == '\0') return 1; /* . is root-ish */
    }
    if (path[0] == '/') path++;

    uint8_t *ptr = initrd_start;
    while (ptr < initrd_start + initrd_size) {
        struct tar_header *header = (struct tar_header *)ptr;
        if (header->filename[0] == '\0') break;
        if (header->magic[0] != 'u') break;

        uint64_t size = octal_to_int(header->size, 11);
        
        if (header->typeflag != 'X') {
            const char *name = normalize_name(header->filename);
            
            char with_slash[256];
            str_cpy(with_slash, path, 254);
            int len = str_len(with_slash);
            if (len > 0 && with_slash[len-1] != '/') {
                with_slash[len] = '/';
                with_slash[len+1] = '\0';
            }
            
            if (str_cmp(name, with_slash) == 0) return 1;
            if (str_cmp(name, path) == 0 && header->typeflag == '5') return 1;
            if (str_starts_with(name, with_slash)) return 1;
        }

        uint64_t total_size = 512 + ((size + 511) / 512) * 512;
        ptr += total_size;
    }
    return 0;
}

struct initrd_file *initrd_find_file(const char *name) {
    static struct initrd_file file;
    if (!initrd_start) return NULL;

    if (name[0] == '/') name++;
    if (name[0] == '.' && name[1] == '/') name += 2;

    uint8_t *ptr = initrd_start;
    struct initrd_file *latest_match = NULL;

    while (ptr < initrd_start + initrd_size) {
        struct tar_header *header = (struct tar_header *)ptr;
        if (header->filename[0] == '\0') break;
        if (header->magic[0] != 'u') break;

        uint64_t size = octal_to_int(header->size, 11);
        
        if (header->typeflag != 'X') {
            const char *hname = normalize_name(header->filename);
            if (str_cmp(hname, name) == 0) {
                str_cpy(file.name, hname, 256);
                file.size = size;
                file.data = ptr + 512;
                latest_match = &file;
            }
        }
        uint64_t total_size = 512 + ((size + 511) / 512) * 512;
        ptr += total_size;
    }
    return latest_match; /* Returns the LAST matching file found (most recent) */
}
