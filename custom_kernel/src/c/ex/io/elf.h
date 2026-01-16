#ifndef ELF_H
#define ELF_H

#include <stdint.h>

/* ELF Magic */
#define ELF_MAGIC 0x464C457F /* "\x7FELF" */

/* ELF File Types */
#define ET_NONE   0  /* No file type */
#define ET_REL    1  /* Relocatable file */
#define ET_EXEC   2  /* Executable file */
#define ET_DYN    3  /* Shared object file */
#define ET_CORE   4  /* Core file */

/* ELF Machine Types */
#define EM_X86_64 62 /* AMD x86-64 */

/* Program Header Types */
#define PT_NULL    0 /* Unused */
#define PT_LOAD    1 /* Loadable segment */
#define PT_DYNAMIC 2 /* Dynamic linking info */
#define PT_INTERP  3 /* Interpreter path */
#define PT_NOTE    4 /* Auxiliary info */
#define PT_SHLIB   5 /* Reserved */
#define PT_PHDR    6 /* Program header table */
#define PT_TLS     7 /* Thread-local storage */

/* Program Header Flags */
#define PF_X 0x1 /* Executable */
#define PF_W 0x2 /* Writable */
#define PF_R 0x4 /* Readable */

/* Section Header Types */
#define SHT_NULL     0  /* Unused */
#define SHT_PROGBITS 1  /* Program data */
#define SHT_SYMTAB   2  /* Symbol table */
#define SHT_STRTAB   3  /* String table */
#define SHT_RELA     4  /* Relocation with addend */
#define SHT_HASH     5  /* Symbol hash table */
#define SHT_DYNAMIC  6  /* Dynamic linking info */
#define SHT_NOTE     7  /* Notes */
#define SHT_NOBITS   8  /* .bss */
#define SHT_REL      9  /* Relocation */

/* ELF64 Header */
typedef struct {
    uint32_t magic;         /* 0x7F followed by ELF(45 4c 46) */
    uint8_t  elf_class;     /* 1 = 32-bit, 2 = 64-bit */
    uint8_t  endian;        /* 1 = little, 2 = big */
    uint8_t  version;       /* ELF version */
    uint8_t  osabi;         /* OS/ABI identification */
    uint8_t  abiversion;    /* ABI version */
    uint8_t  pad[7];        /* Padding */
    uint16_t type;          /* Object file type */
    uint16_t machine;       /* Architecture */
    uint32_t elf_version;   /* ELF version */
    uint64_t entry;         /* Entry point virtual address */
    uint64_t phoff;         /* Program header table file offset */
    uint64_t shoff;         /* Section header table file offset */
    uint32_t flags;         /* Processor-specific flags */
    uint16_t ehsize;        /* ELF header size */
    uint16_t phentsize;     /* Program header entry size */
    uint16_t phnum;         /* Number of program headers */
    uint16_t shentsize;     /* Section header entry size */
    uint16_t shnum;         /* Number of section headers */
    uint16_t shstrndx;      /* Section name string table index */
} __attribute__((packed)) elf64_header_t;

/* ELF64 Program Header */
typedef struct {
    uint32_t type;          /* Segment type */
    uint32_t flags;         /* Segment flags */
    uint64_t offset;        /* Segment file offset */
    uint64_t vaddr;         /* Segment virtual address */
    uint64_t paddr;         /* Segment physical address */
    uint64_t filesz;        /* Segment size in file */
    uint64_t memsz;         /* Segment size in memory */
    uint64_t align;         /* Segment alignment */
} __attribute__((packed)) elf64_phdr_t;

/* ELF64 Section Header */
typedef struct {
    uint32_t name;          /* Section name (index into string table) */
    uint32_t type;          /* Section type */
    uint64_t flags;         /* Section flags */
    uint64_t addr;          /* Section virtual address */
    uint64_t offset;        /* Section file offset */
    uint64_t size;          /* Section size */
    uint32_t link;          /* Link to another section */
    uint32_t info;          /* Additional section info */
    uint64_t addralign;     /* Section alignment */
    uint64_t entsize;       /* Entry size if section holds table */
} __attribute__((packed)) elf64_shdr_t;

/* ELF Load Result */
typedef struct {
    uint64_t entry_point;   /* Program entry point */
    uint64_t stack_top;     /* Top of user stack */
    uint64_t brk;           /* Initial program break */
    void *address_space;    /* VMM address space */
} elf_load_result_t;

/* Functions */
int elf_validate(const uint8_t *data, uint64_t size);
int elf_load(const uint8_t *data, uint64_t size, elf_load_result_t *result);

int elf_load_file(const char *path, elf_load_result_t *result);

#endif
