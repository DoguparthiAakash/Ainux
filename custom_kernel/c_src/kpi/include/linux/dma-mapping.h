/* SPDX-License-Identifier: GPL-2.0 */
/* Ainux KPI — linux/dma-mapping.h
 * DMA coherent memory allocation. On bare-metal x86-64 without IOMMU,
 * physical = virtual - KERNEL_BASE. All DMA allocations come from our
 * physically-contiguous heap.
 */
#ifndef _LINUX_DMA_MAPPING_H
#define _LINUX_DMA_MAPPING_H

#include <linux/types.h>

/* DMA directions */
#define DMA_BIDIRECTIONAL   0
#define DMA_TO_DEVICE       1
#define DMA_FROM_DEVICE     2
#define DMA_NONE            3

/* Device stub — Linux drivers take a 'struct device *'. We use NULL. */
struct device { int dummy; };

/* dma_alloc_coherent — allocate physically contiguous DMA-able memory.
 * Returns virtual address; stores physical address in dma_handle. */
extern void *kpi_dma_alloc_coherent(struct device *dev, size_t size,
                                    dma_addr_t *dma_handle, gfp_t flag);
extern void  kpi_dma_free_coherent(struct device *dev, size_t size,
                                   void *cpu_addr, dma_addr_t dma_handle);
extern dma_addr_t kpi_dma_map_single(struct device *dev, void *ptr,
                                     size_t size, int direction);
extern void kpi_dma_unmap_single(struct device *dev, dma_addr_t dma_addr,
                                 size_t size, int direction);

static __always_inline void *dma_alloc_coherent(struct device *dev, size_t size,
                                                dma_addr_t *dma_handle, gfp_t flag)
{
    return kpi_dma_alloc_coherent(dev, size, dma_handle, flag);
}
static __always_inline void dma_free_coherent(struct device *dev, size_t size,
                                              void *cpu_addr, dma_addr_t dma_handle)
{
    kpi_dma_free_coherent(dev, size, cpu_addr, dma_handle);
}
static __always_inline dma_addr_t dma_map_single(struct device *dev, void *ptr,
                                                  size_t size, int dir)
{
    return kpi_dma_map_single(dev, ptr, size, dir);
}
static __always_inline void dma_unmap_single(struct device *dev, dma_addr_t addr,
                                             size_t size, int dir)
{
    kpi_dma_unmap_single(dev, addr, size, dir);
}

/* On x86 without IOMMU, virt_to_phys is trivial */
#define KERNEL_BASE  0xFFFFFFFF80000000UL
static __always_inline dma_addr_t virt_to_phys(const void *addr)
{
    return (dma_addr_t)((unsigned long)addr - KERNEL_BASE);
}
static __always_inline void *phys_to_virt(dma_addr_t phys)
{
    return (void *)(phys + KERNEL_BASE);
}

#define dma_mapping_error(dev, addr) ((int)(addr) == 0)

#endif /* _LINUX_DMA_MAPPING_H */
