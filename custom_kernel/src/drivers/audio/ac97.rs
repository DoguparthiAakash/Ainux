use crate::drivers::video;
use core::arch::asm;
use lazy_static::lazy_static;
use spin::Mutex;

const AC97_VENDOR_ID: u16 = 0x8086;
const AC97_DEVICE_ID: u16 = 0x2415;

unsafe fn outw(port: u16, val: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack, preserves_flags));
}

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

unsafe fn ind(port: u16) -> u32 {
    let mut val: u32;
    asm!("in eax, dx", out("eax") val, in("dx") port, options(nomem, nostack, preserves_flags));
    val
}

pub struct Ac97 {
    nambar: u16,
    nabmbar: u16,
    initialized: bool,
}

lazy_static! {
    pub static ref AC97: Mutex<Ac97> = Mutex::new(Ac97 { nambar: 0, nabmbar: 0, initialized: false });
}

pub fn init_device(bus: u8, slot: u8, func: u8) {
    let bar0 = crate::drivers::pci::pci_config_read(bus, slot, func, 0x10);
    let bar1 = crate::drivers::pci::pci_config_read(bus, slot, func, 0x14);
    
    // Check if they are Port I/O (bit 0 must be 1)
    if (bar0 & 1) == 0 || (bar1 & 1) == 0 {
        unsafe { video::put_str("AC97: Expected Port I/O BARs but got Memory-Mapped.\n"); }
        return;
    }
    
    let nambar = (bar0 & !3) as u16;
    let nabmbar = (bar1 & !3) as u16;
    
    // Enable bus mastering
    let cmd = crate::drivers::pci::pci_config_read(bus, slot, func, 0x04);
    crate::drivers::pci::pci_config_write(bus, slot, func, 0x04, cmd | (1 << 2)); // Bus Master Enable
    
    unsafe {
        // Reset Mixer
        outw(nambar + 0x00, 0);
        
        // Set Master Volume to 0 (Max)
        outw(nambar + 0x02, 0x0000);
        
        // Set PCM Volume to 0 (Max)
        outw(nambar + 0x18, 0x0000);
        
        video::put_str("AC97: Audio Controller Initialized.\n");
    }
    
    let mut ac97 = AC97.lock();
    ac97.nambar = nambar;
    ac97.nabmbar = nabmbar;
    ac97.initialized = true;
}

pub fn init() {
    // PCI will detect and call init_device
}

#[repr(C, packed)]
struct BdlEntry {
    addr: u32,
    length: u16,
    flags: u16,
}

// Raw audio data extracted from the user's MP3 file (16-bit, 48000Hz, mono)
static STARTUP_SOUND: &[u8] = include_bytes!("../../../Audio/startup.raw");

pub fn play_startup_sound() {
    let mut ac97 = AC97.lock();
    if !ac97.initialized { return; }

    let nabmbar = ac97.nabmbar;
    
    // Stop any ongoing PCM out
    unsafe {
        outb(nabmbar + 0x1B, 0x00);
        outb(nabmbar + 0x1B, 0x02); // Reset
        let mut timeout = 1000;
        while timeout > 0 { core::arch::asm!("pause"); timeout -= 1; }
    }
    
    // Build the BDL
    let bdl_phys = {
        let mut pmm_lock = crate::mm::pmm::PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            pmm.alloc_frame().unwrap()
        } else { return; }
    };
    
    let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
    let bdl_virt = bdl_phys + hhdm;
    let bdl = unsafe { core::slice::from_raw_parts_mut(bdl_virt as *mut BdlEntry, 32) };
    
    let mut chunks: alloc::vec::Vec<(u64, usize)> = alloc::vec::Vec::new();
    let mut offset = 0;
    let size = STARTUP_SOUND.len();
    let vaddr = STARTUP_SOUND.as_ptr() as u64;
    
    while offset < size {
        let current_vaddr = vaddr + offset as u64;
        let phys_addr = unsafe { crate::mm::vmm::virt_to_phys_walk(current_vaddr).unwrap() };
        
        let page_offset = current_vaddr & 0xFFF;
        let mut chunk_size = 4096 - page_offset as usize;
        if chunk_size > size - offset {
            chunk_size = size - offset;
        }
        
        if let Some(last) = chunks.last_mut() {
            if last.0 + last.1 as u64 == phys_addr && last.1 + chunk_size <= 131070 {
                // Combine chunks up to BDL size limit (65535 samples = 131070 bytes)
                last.1 += chunk_size;
            } else {
                chunks.push((phys_addr, chunk_size));
            }
        } else {
            chunks.push((phys_addr, chunk_size));
        }
        offset += chunk_size;
    }
    
    if chunks.len() > 32 {
        video::put_str("AC97: Audio too fragmented for 32 BDL entries!\n");
        return;
    }
    
    for (i, &(phys, chunk_size)) in chunks.iter().enumerate() {
        bdl[i].addr = phys as u32;
        bdl[i].length = (chunk_size / 2) as u16; // Length is in samples
        bdl[i].flags = if i == chunks.len() - 1 { 0x8000 } else { 0 }; // Interrupt On Completion (IOC) on last entry
    }
    
    unsafe {
        // Set BDL Base Address
        asm!("out dx, eax", in("dx") nabmbar + 0x10, in("eax") bdl_phys as u32);
        
        // Set Last Valid Index
        outb(nabmbar + 0x15, (chunks.len() - 1) as u8);
        
        // Start PCM out!
        outb(nabmbar + 0x1B, 0x01);
    }
    
    video::put_str("AC97: Startup sound playback started.\n");
}

pub fn get_speaker_count() -> u32 {
    let ac97 = AC97.lock();
    if !ac97.initialized { return 0; }
    
    // Read Extended Audio ID register (0x28)
    let ext_id = unsafe {
        let mut val: u16;
        asm!("in ax, dx", out("ax") val, in("dx") ac97.nambar + 0x28, options(nomem, nostack, preserves_flags));
        val
    };
    
    // Bit 6: Center/LFE supported (5.1 = 6 speakers)
    // Bit 7: Surround supported (4 speakers)
    if (ext_id & (1 << 6)) != 0 {
        6
    } else if (ext_id & (1 << 7)) != 0 {
        4
    } else {
        2 // Default stereo
    }
}

pub fn play_beep() {
    let mut ac97 = AC97.lock();
    if !ac97.initialized { return; }
    
    // Play a short beep by writing a square wave to PCM out.
    // For simplicity, we just reuse the startup sound logic with a dynamically generated buffer.
    // We will generate a quick square wave.
    
    let nabmbar = ac97.nabmbar;
    unsafe {
        outb(nabmbar + 0x1B, 0x00); // Stop
        outb(nabmbar + 0x1B, 0x02); // Reset
        let mut timeout = 1000;
        while timeout > 0 { core::arch::asm!("pause"); timeout -= 1; }
    }
    
    // Generate square wave in memory
    // (Allocate a small frame for audio data)
    let bdl_phys = {
        let mut pmm_lock = crate::mm::pmm::PMM.lock();
        if let Some(ref mut pmm) = *pmm_lock {
            pmm.alloc_frame().unwrap()
        } else { return; }
    };
    
    let hhdm = crate::mm::pmm::HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
    let page_virt = bdl_phys + hhdm;
    
    // BDL is at start of page, audio data is immediately after it.
    let bdl = unsafe { core::slice::from_raw_parts_mut(page_virt as *mut BdlEntry, 1) };
    let audio_data = unsafe { core::slice::from_raw_parts_mut((page_virt + 128) as *mut i16, 1024) };
    
    for i in 0..1024 {
        audio_data[i] = if (i / 10) % 2 == 0 { 10000 } else { -10000 };
    }
    
    bdl[0].addr = (bdl_phys + 128) as u32;
    bdl[0].length = 1024; // Samples
    bdl[0].flags = 0x8000; // IOC
    
    unsafe {
        asm!("out dx, eax", in("dx") nabmbar + 0x10, in("eax") bdl_phys as u32);
        outb(nabmbar + 0x15, 0); // Last index = 0
        outb(nabmbar + 0x1B, 0x01); // Play
    }
}
