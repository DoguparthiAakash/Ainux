use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::types::{IOValue, IOResult};
use crate::drivers::video;

const VENDOR_ID: i64 = 0x10EC;
const DEVICE_ID: i64 = 0x8139;

// Registers
const MAC0: u16 = 0x00;
const MAR0: u16 = 0x08;
const TX_STATUS0: u16 = 0x10;
const TX_ADDR0: u16 = 0x20;
const RX_BUF: u16 = 0x30;
const COMMAND: u16 = 0x37;
const IMR: u16 = 0x3C;
const ISR: u16 = 0x3E;
const CONFIG1: u16 = 0x52;

#[derive(Debug)]
pub struct RTL8139 {
    name: String,
}

impl RTL8139 {
    pub fn new() -> Self {
        Self { name: String::from("RTL8139 Ethernet") }
    }
    
    // Helper helpers
    unsafe fn outb(port: u16, val: u8) {
        core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
    }
    
    unsafe fn outw(port: u16, val: u16) {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(nostack, preserves_flags));
    }
    
    unsafe fn outl(port: u16, val: u32) {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") val, options(nostack, preserves_flags));
    }
}

impl IOService for RTL8139 {
    fn get_name(&self) -> &str { &self.name }
    
    fn get_property(&self, _key: &str) -> Option<IOValue> { None }
    
    fn probe(&self, provider: &Arc<dyn IOService>) -> i32 {
        if let Some(IOValue::Integer(vid)) = provider.get_property("vendor-id") {
            if let Some(IOValue::Integer(did)) = provider.get_property("device-id") {
                if vid == VENDOR_ID && did == DEVICE_ID {
                    return 100; // Match!
                }
            }
        }
        0
    }
    
    fn start(&self, provider: &Arc<dyn IOService>) -> IOResult<()> {
        video::put_str("RTL8139: Initializing...\n");
        
        let bar0 = match provider.get_property("bar0") {
            Some(IOValue::Integer(b)) => b as u16,
            _ => return Err(crate::drivers::iokit::types::IOError::DeviceError),
        };
        
        let io_base = bar0 & !1;
        video::put_str(&alloc::format!("RTL8139: I/O Base: {:#x}\n", io_base));
        
        // 1. Enable Hardware
        unsafe {
            Self::outb(io_base + CONFIG1, 0x00); // Power On
            Self::outb(io_base + COMMAND, 0x10); // Reset
            let mut timeout = 10000;
            while timeout > 0 { timeout -= 1; }
            
            // 2. Allocate DMA Buffers (RX + 4 TX)
            // We need contiguous physical memory. Using PMM to alloc single frames.
            // RX Buffer (8K + 16 + 1.5K wrap) -> say 3 pages (12K)
            // TX Buffers (2K each x 4) -> 2 pages
            
            // Helper to get phys addr of a frame
            // WARNING: This is a hacky way to get a static buffer for the driver
            // In a real OS, use the driver struct state. Here we use static mut or just leak it.
            
            let mut pmm_lock = crate::mm::pmm::PMM.lock();
            if let Some(pmm) = pmm_lock.as_mut() {
                 let rx_phys = pmm.alloc_frame().unwrap();
                 let tx_phys = pmm.alloc_frame().unwrap(); // 4K is enough for 2 TX buffers? 
                 // Let's alloc one frame per buffer to be safe and simple
                 let tx0_phys = pmm.alloc_frame().unwrap();
                 let tx1_phys = pmm.alloc_frame().unwrap();
                 let tx2_phys = pmm.alloc_frame().unwrap();
                 let tx3_phys = pmm.alloc_frame().unwrap();
                 
                  // Store IO Base globally
                  RTL8139_IO_BASE = io_base;
                  RTL8139_TX_PHYS[0] = tx0_phys;
                  RTL8139_TX_PHYS[1] = tx1_phys;
                  RTL8139_TX_PHYS[2] = tx2_phys;
                  RTL8139_TX_PHYS[3] = tx3_phys;
                  RTL8139_RX_PHYS = rx_phys;
                  
                  video::put_str(&alloc::format!("RTL8139: RX Buffer at {:#x}\n", rx_phys));
                  
                  // 3. Init RX Buffer
                  Self::outl(io_base + RX_BUF, rx_phys as u32);
                  
                  // 4. Init IM/ISR
                  Self::outw(io_base + IMR, 0x0005); // TOK + ROK (Transmit OK, Receive OK)
                  
                  // 5. Build RCR (Receive Config Register)
                  // ABP (Accept Broadcast/Physical/Multicast) | WRAP
                  Self::outl(io_base + 0x44, 0x0F | (1 << 7));
                  
                  // 6. Enable RE/TE
                  Self::outb(io_base + COMMAND, 0x0C);

                  // 7. Unmask IRQ 11 in PIC
                  unsafe { crate::cpu::pic::unmask_irq(11); }
            }
        }
        
        video::put_str("RTL8139: Driver Active (DMA Configured).\n");
        Ok(())
    }
    
    fn stop(&self) {
        video::put_str("RTL8139: Stopping...\n");
    }
}

// Global state for simple access
static mut RTL8139_IO_BASE: u16 = 0;
static mut RTL8139_TX_PHYS: [u64; 4] = [0; 4];
static mut RTL8139_TX_CUR: usize = 0;
static mut RTL8139_RX_PHYS: u64 = 0;
static mut RTL8139_RX_OFFSET: usize = 0;

pub static mut RTL8139_FRAMEWORK: Option<u8> = Some(1); // Marker

impl RTL8139 {
    pub fn send_packet(data: &[u8]) {
        unsafe {
            let io_base = RTL8139_IO_BASE;
            if io_base == 0 { return; }
            
            let cur_tx = RTL8139_TX_CUR;
            let phys_addr = RTL8139_TX_PHYS[cur_tx];
            
            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let virt_addr = phys_addr + hhdm;
            let ptr = virt_addr as *mut u8;
            
            let len = if data.len() > 1792 { 1792 } else { data.len() };
            core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, len);
            
            Self::outl(io_base + 0x20 + (cur_tx as u16 * 4), phys_addr as u32);
            Self::outl(io_base + 0x10 + (cur_tx as u16 * 4), len as u32);
            
            RTL8139_TX_CUR = (cur_tx + 1) % 4;
        }
    }

    pub fn receive_packet<F>(mut handler: F) where F: FnMut(&[u8]) {
        unsafe {
            let io_base = RTL8139_IO_BASE;
            if io_base == 0 { return; }

            // Check if buffer is empty
            if (Self::inb(io_base + COMMAND) & 0x01) != 0 {
                return;
            }

            let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
            let rx_virt = RTL8139_RX_PHYS + hhdm;
            
            while (Self::inb(io_base + COMMAND) & 0x01) == 0 {
                let offset = RTL8139_RX_OFFSET;
                let ptr = (rx_virt + offset as u64) as *const u16;
                
                let header = *ptr;
                let status = header;
                let len = *ptr.add(1);

                if (status & 1) == 0 { break; } // Packet not OK

                let data_ptr = (rx_virt + offset as u64 + 4) as *const u8;
                let packet = core::slice::from_raw_parts(data_ptr, len as usize - 4);
                
                handler(packet);

                // Update offset (aligned to 4 bytes as per RTL8139 spec)
                let mut new_offset = (offset + len as usize + 4 + 3) & !3;
                if new_offset >= 8192 {
                    new_offset %= 8192;
                }
                RTL8139_RX_OFFSET = new_offset;
                Self::outw(io_base + 0x38, (new_offset as i16 - 16) as u16); // CBR
            }
        }
    }

    pub fn handle_interrupt() {
        unsafe {
            let io_base = RTL8139_IO_BASE;
            if io_base == 0 { return; }

            let isr = Self::inw(io_base + ISR);
            if (isr & 0x01) != 0 { // ROK
                // We'll process packets in the network stack processing loop
            }
            
            // Ack all interrupts
            Self::outw(io_base + ISR, isr);
        }
    }

    unsafe fn inb(port: u16) -> u8 {
        let mut val: u8;
        core::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags));
        val
    }

    unsafe fn inw(port: u16) -> u16 {
        let mut val: u16;
        core::arch::asm!("in ax, dx", out("ax") val, in("dx") port, options(nostack, preserves_flags));
        val
    }
}
