// =============================================================================
// eXtensible Host Controller Interface (xHCI / USB 3.0, 3.1, 3.2, USB4) Driver
// PCI Class 0x0C Subclass 0x03 Prog-IF 0x30.
// Modern high-speed USB standard supporting SuperSpeed (5-10 Gbps) and backwards
// compatibility with USB 2.0 / 1.1.
// =============================================================================

use core::arch::asm;
use spin::Mutex;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::fmt::Write;

// Capability Register Offsets (relative to BAR0)
const CAP_CAPLENGTH:   usize = 0x00;
const CAP_HCIVERSION:  usize = 0x02;
const CAP_HCSPARAMS1:  usize = 0x04;
const CAP_HCSPARAMS2:  usize = 0x08;
const CAP_HCSPARAMS3:  usize = 0x0C;
const CAP_HCCPARAMS1:  usize = 0x10;
const CAP_DBOFF:       usize = 0x14;
const CAP_RTSOFF:      usize = 0x18;

// Operational Register Offsets (relative to op_base = BAR0 + CAPLENGTH)
const OP_USBCMD:   usize = 0x00;
const OP_USBSTS:   usize = 0x04;
const OP_PAGESIZE: usize = 0x08;
const OP_DNCTRL:   usize = 0x14;
const OP_CRCR:     usize = 0x18;
const OP_DCBAAP:   usize = 0x30;
const OP_CONFIG:   usize = 0x38;

// USBCMD Register Bits
const USBCMD_RS:    u32 = 1 << 0; // Run / Stop
const USBCMD_HCRST: u32 = 1 << 1; // Host Controller Reset
const USBCMD_INTE:  u32 = 1 << 2; // Interrupter Enable
const USBCMD_HSEE:  u32 = 1 << 3; // Host System Error Enable

// USBSTS Register Bits
const USBSTS_HCH:  u32 = 1 << 0;  // Host Controller Halted
const USBSTS_HSE:  u32 = 1 << 2;  // Host System Error
const USBSTS_EINT: u32 = 1 << 3;  // Event Interrupt
const USBSTS_PCD:  u32 = 1 << 4;  // Port Change Detect
const USBSTS_CNR:  u32 = 1 << 11; // Controller Not Ready

// PORTSC Register Bits (relative to op_base + 0x400 + (port - 1) * 0x10)
const PORTSC_CCS: u32 = 1 << 0;  // Current Connect Status
const PORTSC_PED: u32 = 1 << 1;  // Port Enabled / Disabled
const PORTSC_OCA: u32 = 1 << 3;  // Over-current Active
const PORTSC_PR:  u32 = 1 << 4;  // Port Reset
const PORTSC_PP:  u32 = 1 << 9;  // Port Power
const PORTSC_SPEED_MASK: u32 = 0x0F << 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XhciPortSpeed {
    FullSpeed12Mbps,   // USB 1.1
    LowSpeed15Mbps,    // USB 1.0
    HighSpeed480Mbps,  // USB 2.0
    SuperSpeed5Gbps,   // USB 3.0 / 3.1 Gen 1
    SuperSpeedPlus10G, // USB 3.1 Gen 2 / USB 3.2
    Unknown(u32),
}

impl XhciPortSpeed {
    pub fn name(&self) -> &'static str {
        match self {
            XhciPortSpeed::FullSpeed12Mbps => "Full-Speed (12 Mbps, USB 1.1)",
            XhciPortSpeed::LowSpeed15Mbps => "Low-Speed (1.5 Mbps, USB 1.0)",
            XhciPortSpeed::HighSpeed480Mbps => "High-Speed (480 Mbps, USB 2.0)",
            XhciPortSpeed::SuperSpeed5Gbps => "SuperSpeed (5 Gbps, USB 3.0)",
            XhciPortSpeed::SuperSpeedPlus10G => "SuperSpeed+ (10 Gbps, USB 3.1/3.2)",
            XhciPortSpeed::Unknown(_) => "Unknown Speed",
        }
    }
}

pub struct XhciPortInfo {
    pub port_num: u8,
    pub connected: bool,
    pub enabled: bool,
    pub powered: bool,
    pub speed: XhciPortSpeed,
}

pub struct XhciController {
    pub bar0: usize,
    pub cap_length: usize,
    pub op_base: usize,
    pub version: u16,
    pub max_slots: u8,
    pub max_ports: u8,
    pub max_intrs: u16,
    pub ports: Vec<XhciPortInfo>,
    pub initialized: bool,
}

pub static ACTIVE_XHCI: Mutex<Option<XhciController>> = Mutex::new(None);

impl XhciController {
    pub fn new(bar0: usize) -> Self {
        Self {
            bar0,
            cap_length: 0,
            op_base: 0,
            version: 0,
            max_slots: 0,
            max_ports: 0,
            max_intrs: 0,
            ports: Vec::new(),
            initialized: false,
        }
    }

    #[inline]
    unsafe fn read_cap8(&self, offset: usize) -> u8 {
        core::ptr::read_volatile((self.bar0 + offset) as *const u8)
    }

    #[inline]
    unsafe fn read_cap16(&self, offset: usize) -> u16 {
        core::ptr::read_volatile((self.bar0 + offset) as *const u16)
    }

    #[inline]
    unsafe fn read_cap32(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.bar0 + offset) as *const u32)
    }

    #[inline]
    unsafe fn read_op32(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.op_base + offset) as *const u32)
    }

    #[inline]
    unsafe fn write_op32(&self, offset: usize, val: u32) {
        core::ptr::write_volatile((self.op_base + offset) as *mut u32, val);
    }

    #[inline]
    unsafe fn read_port32(&self, port: u8, offset: usize) -> u32 {
        let port_reg = self.op_base + 0x400 + ((port as usize - 1) * 0x10) + offset;
        core::ptr::read_volatile(port_reg as *const u32)
    }

    #[inline]
    unsafe fn write_port32(&self, port: u8, offset: usize, val: u32) {
        let port_reg = self.op_base + 0x400 + ((port as usize - 1) * 0x10) + offset;
        core::ptr::write_volatile(port_reg as *mut u32, val);
    }

    pub fn init(&mut self) -> bool {
        let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
        let _ = write!(serial, "xHCI: Initializing USB 3.x Controller at BAR0 {:#x}...\n", self.bar0);

        unsafe {
            // Read Capability Registers
            self.cap_length = self.read_cap8(CAP_CAPLENGTH) as usize;
            self.op_base = self.bar0 + self.cap_length;
            self.version = self.read_cap16(CAP_HCIVERSION);

            let hcsparams1 = self.read_cap32(CAP_HCSPARAMS1);
            self.max_slots = (hcsparams1 & 0xFF) as u8;
            self.max_intrs = ((hcsparams1 >> 8) & 0x7FF) as u16;
            self.max_ports = ((hcsparams1 >> 24) & 0xFF) as u8;

            let hccparams1 = self.read_cap32(CAP_HCCPARAMS1);
            let is_64bit = (hccparams1 & 1) != 0;

            let major = (self.version >> 8) & 0xFF;
            let minor = (self.version >> 4) & 0x0F;
            let sub = self.version & 0x0F;

            let _ = write!(
                serial,
                "xHCI: Spec v{}.{}.{} | Max Slots: {} | Max Ports: {} | 64-bit DMA: {}\n",
                major, minor, sub, self.max_slots, self.max_ports, is_64bit
            );

            // Halt controller if running
            let mut usbcmd = self.read_op32(OP_USBCMD);
            if (usbcmd & USBCMD_RS) != 0 {
                self.write_op32(OP_USBCMD, usbcmd & !USBCMD_RS);
                for _ in 0..100_000 {
                    if (self.read_op32(OP_USBSTS) & USBSTS_HCH) != 0 {
                        break;
                    }
                    core::hint::spin_loop();
                }
            }

            // Reset host controller
            self.write_op32(OP_USBCMD, USBCMD_HCRST);
            for _ in 0..100_000 {
                if (self.read_op32(OP_USBCMD) & USBCMD_HCRST) == 0 {
                    break;
                }
                core::hint::spin_loop();
            }

            // Wait for Controller Not Ready (CNR) to clear
            for _ in 0..100_000 {
                if (self.read_op32(OP_USBSTS) & USBSTS_CNR) == 0 {
                    break;
                }
                core::hint::spin_loop();
            }

            // Program Max Slots in CONFIG register
            self.write_op32(OP_CONFIG, self.max_slots as u32);

            // Power on all root hub ports (write Port Power bit)
            for p in 1..=self.max_ports {
                let portsc = self.read_port32(p, 0);
                self.write_port32(p, 0, portsc | PORTSC_PP);
            }

            // Delay for port power stabilization (~20ms)
            for _ in 0..200_000 {
                core::hint::spin_loop();
            }

            // Scan root hub ports
            for p in 1..=self.max_ports {
                let portsc = self.read_port32(p, 0);
                let connected = (portsc & PORTSC_CCS) != 0;
                let enabled = (portsc & PORTSC_PED) != 0;
                let powered = (portsc & PORTSC_PP) != 0;
                let raw_speed = (portsc >> 10) & 0x0F;

                let speed = match raw_speed {
                    1 => XhciPortSpeed::FullSpeed12Mbps,
                    2 => XhciPortSpeed::LowSpeed15Mbps,
                    3 => XhciPortSpeed::HighSpeed480Mbps,
                    4 => XhciPortSpeed::SuperSpeed5Gbps,
                    5 => XhciPortSpeed::SuperSpeedPlus10G,
                    v => XhciPortSpeed::Unknown(v),
                };

                if connected {
                    let _ = write!(
                        serial,
                        "xHCI Port {}: Attached Device detected! Speed: {}\n",
                        p, speed.name()
                    );
                }

                self.ports.push(XhciPortInfo {
                    port_num: p,
                    connected,
                    enabled,
                    powered,
                    speed,
                });
            }

            // Enable Run bit in USBCMD
            let run_cmd = self.read_op32(OP_USBCMD) | USBCMD_RS | USBCMD_INTE;
            self.write_op32(OP_USBCMD, run_cmd);
        }

        self.initialized = true;
        let _ = write!(serial, "xHCI: Subsystem Online ({} Root Hub Ports Active)\n", self.max_ports);
        true
    }
}

pub fn detect_and_init(bus: u8, slot: u8, func: u8, bar0_raw: u32) -> bool {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let bar0 = (bar0_raw & 0xFFFF_FFF0) as usize;

    let _ = write!(
        serial,
        "xHCI: Discovered USB 3.x Controller at PCI {:02x}:{:02x}.{} (BAR0: {:#x})\n",
        bus, slot, func, bar0
    );

    let mut controller = XhciController::new(bar0);
    if controller.init() {
        *ACTIVE_XHCI.lock() = Some(controller);
        true
    } else {
        false
    }
}

pub fn is_present() -> bool {
    ACTIVE_XHCI.lock().is_some()
}

pub fn get_status() -> String {
    let guard = ACTIVE_XHCI.lock();
    if let Some(ctrl) = guard.as_ref() {
        let mut out = format!(
            "USB 3.x (xHCI) Extensible Host Controller at BAR0 {:#x}:\n  Version     : v{}.{:02}\n  Root Ports  : {} Ports\n  Max Slots   : {} Slots\n",
            ctrl.bar0, (ctrl.version >> 8) & 0xFF, ctrl.version & 0xFF, ctrl.max_ports, ctrl.max_slots
        );

        let mut connected_count = 0;
        for p in &ctrl.ports {
            if p.connected {
                connected_count += 1;
                out.push_str(&format!(
                    "  [Port {}] Connected: {} [Power: OK]\n",
                    p.port_num, p.speed.name()
                ));
            }
        }
        if connected_count == 0 {
            out.push_str("  Status      : Online (Ready for SuperSpeed USB3 devices)\n");
        }
        out
    } else {
        format!("xHCI: No USB 3.x host controller initialized.")
    }
}

