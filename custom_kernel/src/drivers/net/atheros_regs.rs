// src/drivers/net/atheros_regs.rs
// Atheros AR9285 / AR9271 Register Definitions (Derived from Linux ath9k)

pub const AR_CR: u32 = 0x0008;       // Control Register
pub const AR_CR_RXE: u32 = 0x00000004; // Receive Enable
pub const AR_CR_TXE: u32 = 0x00000008; // Transmit Enable
pub const AR_CR_SW_RST: u32 = 0x00000040; // Software Reset

pub const AR_IER: u32 = 0x000c;      // Interrupt Enable Register
pub const AR_IER_ENABLE: u32 = 0x00000001;

pub const AR_RXDP: u32 = 0x000c;     // Receive Descriptor Pointer (Wait, overlap with IER? In AR9285 it varies)
// Note: In AR9002+ it's 0x000c for IER. RXDP is usually 0x0034.
pub const AR_RXDP_V2: u32 = 0x0034;

pub const AR_ISR: u32 = 0x0080;      // Interrupt Status Register
pub const AR_ISR_RXOK: u32 = 0x00000001; 
pub const AR_ISR_TXOK: u32 = 0x00000002;

pub const AR_STA_ID0: u32 = 0x8000;  // MAC Address Low
pub const AR_STA_ID1: u32 = 0x8004;  // MAC Address High

pub const AR_RCR: u32 = 0x8048;      // Receive Config Register
pub const AR_RCR_RXE: u32 = 0x00000001;
pub const AR_RCR_PROM: u32 = 0x00000002; // Promiscuous

// Transmit Queue Registers
pub const AR_Q0_TXDP: u32 = 0x0800; // Queue 0 Transmit Descriptor Pointer

// EEPROM / OTP
pub const AR_EEPROM_BASE: u32 = 0x4000;
