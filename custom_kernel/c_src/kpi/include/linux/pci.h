/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/pci.h
 * PCI bus interface for Linux drivers. Backed by our existing c_src/drivers/pci.c.
 */
#ifndef _LINUX_PCI_H
#define _LINUX_PCI_H

#include <linux/types.h>
#include <linux/io.h>

/* PCI config space registers */
#define PCI_VENDOR_ID           0x00
#define PCI_DEVICE_ID           0x02
#define PCI_COMMAND             0x04
#define PCI_STATUS              0x06
#define PCI_CLASS_REVISION      0x08
#define PCI_CLASS_CODE          0x0B
#define PCI_SUBCLASS_CODE       0x0A
#define PCI_CACHE_LINE_SIZE     0x0C
#define PCI_LATENCY_TIMER       0x0D
#define PCI_HEADER_TYPE         0x0E
#define PCI_BASE_ADDRESS_0      0x10
#define PCI_BASE_ADDRESS_1      0x14
#define PCI_BASE_ADDRESS_2      0x18
#define PCI_BASE_ADDRESS_3      0x1C
#define PCI_BASE_ADDRESS_4      0x20
#define PCI_BASE_ADDRESS_5      0x24
#define PCI_INTERRUPT_LINE      0x3C
#define PCI_INTERRUPT_PIN       0x3D

/* PCI BAR flags */
#define PCI_BASE_ADDRESS_SPACE          0x01
#define PCI_BASE_ADDRESS_SPACE_IO       0x01
#define PCI_BASE_ADDRESS_SPACE_MEMORY   0x00
#define PCI_BASE_ADDRESS_MEM_MASK       (~0x0fUL)
#define PCI_BASE_ADDRESS_IO_MASK        (~0x03UL)
#define PCI_BASE_ADDRESS_MEM_TYPE_64    0x04

/* PCI command register bits */
#define PCI_COMMAND_IO          0x0001
#define PCI_COMMAND_MEMORY      0x0002
#define PCI_COMMAND_MASTER      0x0004
#define PCI_COMMAND_INTX_DISABLE 0x0400

/* IRQ constants */
#define PCI_IRQ_LEGACY          (1 << 0)
#define PCI_IRQ_MSI             (1 << 1)
#define PCI_IRQ_MSIX            (1 << 2)
#define PCI_IRQ_ALL_TYPES       (PCI_IRQ_LEGACY | PCI_IRQ_MSI | PCI_IRQ_MSIX)

/* Resource types */
#define IORESOURCE_MEM  0x00000200
#define IORESOURCE_IO   0x00000100
#define IORESOURCE_IRQ  0x00000400

struct resource {
    resource_size_t start;
    resource_size_t end;
    unsigned long   flags;
};

struct pci_device_id {
    u32 vendor;
    u32 device;
    u32 subvendor;
    u32 subdevice;
    u32 class;
    u32 class_mask;
    unsigned long driver_data;
};
#define PCI_ANY_ID      (~0u)
#define PCI_DEVICE(vend, dev)  \
    .vendor = (vend), .device = (dev), \
    .subvendor = PCI_ANY_ID, .subdevice = PCI_ANY_ID

/* Minimal pci_dev — contains what most drivers actually read */
struct pci_dev {
    u16 vendor;
    u16 device;
    u16 subsystem_vendor;
    u16 subsystem_device;
    u8  revision;
    u8  irq;
    u8  bus;
    u8  devfn;
    struct resource resource[7]; /* BAR0-5 + ROM */
    void   *driver_data;         /* driver private */
};

struct pci_driver {
    const char              *name;
    const struct pci_device_id *id_table;
    int  (*probe)(struct pci_dev *dev, const struct pci_device_id *id);
    void (*remove)(struct pci_dev *dev);
};

/* PCI config space accessors — use our existing pci.c implementation */
extern int  kpi_pci_read_config_byte(struct pci_dev *dev, int where, u8 *val);
extern int  kpi_pci_read_config_word(struct pci_dev *dev, int where, u16 *val);
extern int  kpi_pci_read_config_dword(struct pci_dev *dev, int where, u32 *val);
extern int  kpi_pci_write_config_byte(struct pci_dev *dev, int where, u8 val);
extern int  kpi_pci_write_config_word(struct pci_dev *dev, int where, u16 val);
extern int  kpi_pci_write_config_dword(struct pci_dev *dev, int where, u32 val);

static __always_inline int pci_read_config_byte(struct pci_dev *d, int w, u8  *v) { return kpi_pci_read_config_byte(d,w,v); }
static __always_inline int pci_read_config_word(struct pci_dev *d, int w, u16 *v) { return kpi_pci_read_config_word(d,w,v); }
static __always_inline int pci_read_config_dword(struct pci_dev *d, int w, u32 *v){ return kpi_pci_read_config_dword(d,w,v); }
static __always_inline int pci_write_config_byte(struct pci_dev *d, int w, u8  v) { return kpi_pci_write_config_byte(d,w,v); }
static __always_inline int pci_write_config_word(struct pci_dev *d, int w, u16 v) { return kpi_pci_write_config_word(d,w,v); }
static __always_inline int pci_write_config_dword(struct pci_dev *d, int w, u32 v){ return kpi_pci_write_config_dword(d,w,v); }

extern int   kpi_pci_enable_device(struct pci_dev *dev);
extern void  kpi_pci_disable_device(struct pci_dev *dev);
extern int   kpi_pci_request_regions(struct pci_dev *dev, const char *name);
extern void  kpi_pci_release_regions(struct pci_dev *dev);
extern void  kpi_pci_set_master(struct pci_dev *dev);
extern void  kpi_pci_clear_master(struct pci_dev *dev);
extern int   kpi_pci_alloc_irq_vectors(struct pci_dev *dev, unsigned int min_vecs,
                                       unsigned int max_vecs, unsigned int flags);
extern void  kpi_pci_free_irq_vectors(struct pci_dev *dev);
extern int   kpi_pci_irq_vector(struct pci_dev *dev, unsigned int nr);

static __always_inline int  pci_enable_device(struct pci_dev *d)  { return kpi_pci_enable_device(d); }
static __always_inline void pci_disable_device(struct pci_dev *d) { kpi_pci_disable_device(d); }
static __always_inline int  pci_request_regions(struct pci_dev *d, const char *n) { return kpi_pci_request_regions(d,n); }
static __always_inline void pci_release_regions(struct pci_dev *d)               { kpi_pci_release_regions(d); }
static __always_inline void pci_set_master(struct pci_dev *d)                    { kpi_pci_set_master(d); }
static __always_inline void pci_clear_master(struct pci_dev *d)                  { kpi_pci_clear_master(d); }
static __always_inline int  pci_alloc_irq_vectors(struct pci_dev *d, unsigned int mn, unsigned int mx, unsigned int f)
                                                                                   { return kpi_pci_alloc_irq_vectors(d,mn,mx,f); }
static __always_inline void pci_free_irq_vectors(struct pci_dev *d)               { kpi_pci_free_irq_vectors(d); }
static __always_inline int  pci_irq_vector(struct pci_dev *d, unsigned int nr)   { return kpi_pci_irq_vector(d,nr); }

/* pci_iomap / pci_iounmap */
extern void __iomem *kpi_pci_iomap(struct pci_dev *dev, int bar, unsigned long maxlen);
static __always_inline void __iomem *pci_iomap(struct pci_dev *d, int b, unsigned long m) { return kpi_pci_iomap(d,b,m); }
static __always_inline void          pci_iounmap(struct pci_dev *d, void __iomem *a)       { (void)d; kpi_iounmap(a); }

/* Driver data helpers */
static __always_inline void *pci_get_drvdata(struct pci_dev *pdev)           { return pdev->driver_data; }
static __always_inline void  pci_set_drvdata(struct pci_dev *pdev, void *data){ pdev->driver_data = data; }

/* Resource helpers */
static __always_inline resource_size_t pci_resource_start(struct pci_dev *d, int bar) { return d->resource[bar].start; }
static __always_inline resource_size_t pci_resource_end(struct pci_dev *d, int bar)   { return d->resource[bar].end; }
static __always_inline resource_size_t pci_resource_len(struct pci_dev *d, int bar)   { return d->resource[bar].end - d->resource[bar].start + 1; }
static __always_inline unsigned long   pci_resource_flags(struct pci_dev *d, int bar) { return d->resource[bar].flags; }

#endif /* _LINUX_PCI_H */
