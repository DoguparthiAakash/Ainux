// src/drivers/net/ralink_regs.rs
// Ralink RT2860 / RT3090 PCIe Register Map

pub const MAC_CSR0: u32 = 0x1000;       // ASIC Version / ID
pub const MAC_ADDR_DW0: u32 = 0x1008;  // MAC Address (0-31)
pub const MAC_ADDR_DW1: u32 = 0x100C;  // MAC Address (32-47)

pub const H2M_MAILBOX_CSR: u32 = 0x0408; // Host to MCU Mailbox
pub const MCU_CMD_RES: u32 = 0x040c;    // MCU Command Result

pub const WPDMA_GLO_CFG: u32 = 0x0208;  // DMA Engine Control
pub const TX_BASE_PTR0: u32 = 0x0200;   // TX Queue 0 Base
pub const RX_BASE_PTR: u32 = 0x0210;    // RX Queue Base

pub const ASIC_SW_RES_REG: u32 = 0x0008; // Software Reset
pub const ASIC_SW_RES_BBP: u32 = 0x00000002;
pub const ASIC_SW_RES_RT: u32 = 0x00000001;

// FW Loading Offsets
pub const FW_IMAGE_BASE: u32 = 0x2000;  // Offset in Internal RAM for FW
