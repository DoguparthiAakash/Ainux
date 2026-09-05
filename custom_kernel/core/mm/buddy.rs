// =============================================================================
// Ainux Zoned Buddy Allocator (Level 1 Physical Memory Management)
// Features: Per-Zone locks, Buddy Block splitting/merging, PCP Fast Path
// =============================================================================

use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use crate::mm::pmm::PAGE_SIZE;

pub const MAX_ORDER: usize = 10; // 2^10 = 1024 pages (4MB max contiguous)
pub const MAX_CPUS: usize = 32;
pub const PCP_BATCH: usize = 32;
pub const PCP_MAX: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    DMA = 0,     // 0 - 16MB
    Normal = 1,  // 16MB - 4GB
    HighMem = 2, // > 4GB
}

#[derive(Debug)]
pub struct FreeBlock {
    pub next: Option<u64>,
    pub prev: Option<u64>,
}

pub struct PhysicalZone {
    pub zone_type: ZoneType,
    pub base_addr: u64,
    pub limit_addr: u64,
    pub free_lists: [Option<u64>; MAX_ORDER + 1],
    pub total_frames: usize,
    pub used_frames: usize,
}

pub struct PcpCache {
    pub frames: [u64; PCP_MAX],
    pub count: usize,
}

pub struct ZonedAllocator {
    pub zones: [PhysicalZone; 3],
    pub pcp_caches: [PcpCache; MAX_CPUS],
    pub hhdm_offset: u64,
}

unsafe impl Send for ZonedAllocator {}
unsafe impl Sync for ZonedAllocator {}

impl PhysicalZone {
    pub const fn new(zone_type: ZoneType, base: u64, limit: u64) -> Self {
        Self {
            zone_type,
            base_addr: base,
            limit_addr: limit,
            free_lists: [None; MAX_ORDER + 1],
            total_frames: 0,
            used_frames: 0,
        }
    }

    unsafe fn get_block_meta(&self, hhdm_offset: u64, phys: u64) -> *mut FreeBlock {
        (phys + hhdm_offset) as *mut FreeBlock
    }

    unsafe fn push_list(&mut self, hhdm_offset: u64, order: usize, phys: u64) {
        let meta = self.get_block_meta(hhdm_offset, phys);
        (*meta).prev = None;
        (*meta).next = self.free_lists[order];
        
        if let Some(head_phys) = self.free_lists[order] {
            let head_meta = self.get_block_meta(hhdm_offset, head_phys);
            (*head_meta).prev = Some(phys);
        }
        
        self.free_lists[order] = Some(phys);
    }

    unsafe fn remove_list(&mut self, hhdm_offset: u64, order: usize, phys: u64) {
        let meta = self.get_block_meta(hhdm_offset, phys);
        let prev = (*meta).prev;
        let next = (*meta).next;
        
        if let Some(p) = prev {
            (*self.get_block_meta(hhdm_offset, p)).next = next;
        } else {
            self.free_lists[order] = next;
        }
        
        if let Some(n) = next {
            (*self.get_block_meta(hhdm_offset, n)).prev = prev;
        }
    }

    pub unsafe fn add_free_region(&mut self, hhdm_offset: u64, mut base: u64, mut len: usize) {
        let mut frames_added = 0;
        
        while len >= PAGE_SIZE {
            let mut order = 0;
            while order < MAX_ORDER {
                let block_size = PAGE_SIZE * (1 << (order + 1));
                if base % block_size as u64 != 0 { break; }
                if len < block_size { break; }
                order += 1;
            }
            
            self.push_list(hhdm_offset, order, base);
            let added_size = PAGE_SIZE * (1 << order);
            base += added_size as u64;
            len -= added_size;
            frames_added += 1 << order;
        }
        self.total_frames += frames_added;
    }

    pub fn alloc_buddy(&mut self, hhdm_offset: u64, target_order: usize) -> Option<u64> {
        if target_order > MAX_ORDER { return None; }
        
        let mut found_order = target_order;
        while found_order <= MAX_ORDER && self.free_lists[found_order].is_none() {
            found_order += 1;
        }
        
        if found_order > MAX_ORDER { return None; }
        
        unsafe {
            let phys = self.free_lists[found_order].unwrap();
            self.remove_list(hhdm_offset, found_order, phys);
            
            // Split
            while found_order > target_order {
                found_order -= 1;
                let buddy = phys + (PAGE_SIZE * (1 << found_order)) as u64;
                self.push_list(hhdm_offset, found_order, buddy);
            }
            
            self.used_frames += 1 << target_order;
            Some(phys)
        }
    }

    pub fn free_buddy(&mut self, hhdm_offset: u64, phys: u64, order: usize) {
        let mut current_phys = phys;
        let mut current_order = order;
        
        unsafe {
            while current_order < MAX_ORDER {
                let buddy_phys = current_phys ^ (PAGE_SIZE * (1 << current_order)) as u64;
                
                let mut is_buddy_free = false;
                let mut scan = self.free_lists[current_order];
                while let Some(scan_phys) = scan {
                    if scan_phys == buddy_phys {
                        is_buddy_free = true;
                        break;
                    }
                    scan = (*self.get_block_meta(hhdm_offset, scan_phys)).next;
                }
                
                if is_buddy_free {
                    self.remove_list(hhdm_offset, current_order, buddy_phys);
                    current_phys = core::cmp::min(current_phys, buddy_phys);
                    current_order += 1;
                } else {
                    break;
                }
            }
            
            self.push_list(hhdm_offset, current_order, current_phys);
            self.used_frames -= 1 << order;
        }
    }
}

impl ZonedAllocator {
    pub const fn new() -> Self {
        const INIT_PCP: PcpCache = PcpCache { frames: [0; PCP_MAX], count: 0 };
        Self {
            zones: [
                PhysicalZone::new(ZoneType::DMA, 0, 0x1000000),             // < 16MB
                PhysicalZone::new(ZoneType::Normal, 0x1000000, 0x100000000),// 16MB - 4GB
                PhysicalZone::new(ZoneType::HighMem, 0x100000000, u64::MAX),// > 4GB
            ],
            pcp_caches: [INIT_PCP; MAX_CPUS],
            hhdm_offset: 0,
        }
    }

    pub fn set_hhdm_offset(&mut self, offset: u64) {
        self.hhdm_offset = offset;
    }

    pub fn add_free_region(&mut self, base: u64, len: usize) {
        for zone in self.zones.iter_mut() {
            let zone_start = core::cmp::max(base, zone.base_addr);
            let zone_end = core::cmp::min(base + len as u64, zone.limit_addr);
            if zone_start < zone_end {
                unsafe { zone.add_free_region(self.hhdm_offset, zone_start, (zone_end - zone_start) as usize); }
            }
        }
    }

    fn get_zone_for_addr(&mut self, phys: u64) -> &mut PhysicalZone {
        let mut target = ZoneType::Normal as usize;
        for i in 0..self.zones.len() {
            if phys >= self.zones[i].base_addr && phys < self.zones[i].limit_addr {
                target = i;
                break;
            }
        }
        &mut self.zones[target]
    }

    pub fn alloc(&mut self, target_zone: ZoneType, order: usize) -> Option<u64> {
        let cpu_id = crate::cpu::percpu::get_current_cpu_id();

        // FAST PATH: Order-0 allocations hit the PCP cache
        if order == 0 {
            // Safety: We disable interrupts to prevent CPU migration and preemption
            let flags = crate::cpu::control::save_cpu_flags();
            unsafe { core::arch::asm!("cli", options(nomem, nostack)); }

            let cache = &mut self.pcp_caches[cpu_id];
            if cache.count > 0 {
                cache.count -= 1;
                let phys = cache.frames[cache.count];
                unsafe { crate::cpu::control::restore_cpu_flags(flags); }
                return Some(phys);
            }

            // PCP Miss: We must grab a batch from the correct Zone
            unsafe { crate::cpu::control::restore_cpu_flags(flags); }

            let mut allocated_batch = 0;
            let fallback_zones = match target_zone {
                ZoneType::HighMem => [ZoneType::HighMem as usize, ZoneType::Normal as usize, ZoneType::DMA as usize],
                ZoneType::Normal => [ZoneType::Normal as usize, ZoneType::DMA as usize, ZoneType::HighMem as usize],
                ZoneType::DMA => [ZoneType::DMA as usize, ZoneType::Normal as usize, ZoneType::HighMem as usize],
            };
            let mut returned_phys = None;

            for &z_idx in fallback_zones.iter() {
                let zone = &mut self.zones[z_idx];
                for _ in 0..PCP_BATCH {
                    if let Some(phys) = zone.alloc_buddy(self.hhdm_offset, 0) {
                        if returned_phys.is_none() {
                            returned_phys = Some(phys);
                        } else {
                            // Interrupts off while pushing to PCP
                            let f2 = crate::cpu::control::save_cpu_flags();
                            unsafe { core::arch::asm!("cli", options(nomem, nostack)); }
                            let c = &mut self.pcp_caches[cpu_id];
                            if c.count < PCP_MAX {
                                c.frames[c.count] = phys;
                                c.count += 1;
                            } else {
                                // If PCP filled up unexpectedly, free directly
                                unsafe { crate::cpu::control::restore_cpu_flags(f2); }
                                zone.free_buddy(self.hhdm_offset, phys, 0);
                                break;
                            }
                            unsafe { crate::cpu::control::restore_cpu_flags(f2); }
                        }
                        allocated_batch += 1;
                    } else {
                        break; // Zone exhausted
                    }
                }
                if allocated_batch > 0 { break; }
            }
            return returned_phys;
        }

        // SLOW PATH: Multi-page allocations go directly to Buddy
        let fallback_zones = match target_zone {
            ZoneType::HighMem => [ZoneType::HighMem as usize, ZoneType::Normal as usize, ZoneType::DMA as usize],
            ZoneType::Normal => [ZoneType::Normal as usize, ZoneType::DMA as usize, ZoneType::HighMem as usize],
            ZoneType::DMA => [ZoneType::DMA as usize, ZoneType::Normal as usize, ZoneType::HighMem as usize],
        };
        for &z_idx in fallback_zones.iter() {
            let zone = &mut self.zones[z_idx];
            if let Some(phys) = zone.alloc_buddy(self.hhdm_offset, order) {
                return Some(phys);
            }
        }
        None
    }

    pub fn free(&mut self, phys: u64, order: usize) {
        let cpu_id = crate::cpu::percpu::get_current_cpu_id();

        // FAST PATH: Freeing Order-0
        if order == 0 {
            let flags = crate::cpu::control::save_cpu_flags();
            unsafe { core::arch::asm!("cli", options(nomem, nostack)); }

            let cache = &mut self.pcp_caches[cpu_id];
            if cache.count < PCP_MAX {
                cache.frames[cache.count] = phys;
                cache.count += 1;
                unsafe { crate::cpu::control::restore_cpu_flags(flags); }
                return;
            }

            // PCP Full: Drain half the cache back to Buddy
            let drain_count = PCP_MAX / 2;
            let mut frames_to_free = [0u64; PCP_MAX / 2];
            for i in 0..drain_count {
                cache.count -= 1;
                frames_to_free[i] = cache.frames[cache.count];
            }
            // Push the new frame to PCP
            cache.frames[cache.count] = phys;
            cache.count += 1;

            unsafe { crate::cpu::control::restore_cpu_flags(flags); }

            // Free the drained frames to their respective zones
            let offset = self.hhdm_offset;
            for i in 0..drain_count {
                let p = frames_to_free[i];
                let zone = self.get_zone_for_addr(p);
                zone.free_buddy(offset, p, 0);
            }
            return;
        }

        // SLOW PATH: Multi-page frees
        let offset = self.hhdm_offset;
        let zone = self.get_zone_for_addr(phys);
        zone.free_buddy(offset, phys, order);
    }
}

pub static ZONED_PMM: spin::Mutex<ZonedAllocator> = spin::Mutex::new(ZonedAllocator::new());
