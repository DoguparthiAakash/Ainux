use alloc::vec::Vec;
use alloc::string::String;
use crate::drivers::video;

const OP_PUSH: u8 = 0x01;
const OP_POP: u8 = 0x02;
const OP_ADD: u8 = 0x10;
const OP_SUB: u8 = 0x11;
const OP_DRAW_RECT: u8 = 0x20;
const OP_DRAW_IMG: u8 = 0x21; // Stub
const OP_SLEEP: u8 = 0x30;
const OP_DEBUG_PRINT: u8 = 0x50;
const OP_PRINT_CHAR: u8 = 0x51;
const OP_INPUT: u8 = 0x52;
const OP_PEEK: u8 = 0x40;
const OP_POKE: u8 = 0x41;
const OP_PEEK8: u8 = 0x42;
const OP_POKE8: u8 = 0x43;
const OP_JMP: u8 = 0x60;
const OP_JE: u8 = 0x61;
const OP_CALL: u8 = 0x70;
const OP_RET: u8 = 0x71;
const OP_KERNEL_OP: u8 = 0x80;
const OP_EXIT: u8 = 0xFF;

pub struct NuxVm {
    stack: Vec<i64>,
    ip: usize,
    code: Vec<u8>,
    call_stack: Vec<usize>,
    running: bool,
}

impl NuxVm {
    pub fn new(code: Vec<u8>) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            ip: 0,
            code,
            call_stack: Vec::with_capacity(32),
            running: false,
        }
    }

    pub fn run(&mut self) {
        // Verify Header "ANUX" + Version (u16) + Padding(58) = 64 bytes
        if self.code.len() < 64 {
            video::put_str("NuxVM: Invalid Binary (Too Small)\n");
            return;
        }
        
        if &self.code[0..4] != b"ANUX" {
             video::put_str("NuxVM: Invalid Magic\n");
             return;
        }

        self.ip = 64; // Start after header
        self.running = true;

        video::put_str("NuxVM: Started.\n");

        while self.running && self.ip < self.code.len() {
            let op = self.code[self.ip];
            self.ip += 1;

            match op {
                OP_PUSH => {
                    let val = self.read_i64();
                    self.push(val);
                },
                OP_POP => { self.pop(); },
                OP_ADD => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(a.wrapping_add(b));
                },
                OP_SUB => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(a.wrapping_sub(b));
                },
                OP_DRAW_RECT => {
                    let color = self.pop();
                    let h = self.pop();
                    let w = self.pop();
                    let y = self.pop();
                    let x = self.pop();
                    // Basic Validation
                    if x >= 0 && y >= 0 && w > 0 && h > 0 {
                         video::draw_rect(x as usize, y as usize, w as usize, h as usize, color as u32);
                    }
                },
                OP_SLEEP => {
                    let ms = self.pop();
                    if ms > 0 {
                        unsafe { 
                            let end = crate::process::scheduler::get_ticks() + (ms as u64 / 10); // Assume 1 tick = 10ms? Or 1ms? 
                            // If tick is 10ms, then ms/10.
                            crate::process::scheduler::set_current_sleep(end);
                        }
                        crate::process::scheduler::yield_now();
                    }
                },
                OP_DEBUG_PRINT => { // Not in spec, but good for debug
                     let val = self.pop();
                     // shell::print_digit (not pub). We need a better way.
                     // For now ignore or implement char print
                },
                OP_PRINT_CHAR => {
                    let val = self.pop();
                    unsafe {
                        video::put_char(val as u8 as char);
                    }
                },
                OP_INPUT => {
                   loop {
                        if let Some(c) = crate::drivers::keyboard::pop_char() {
                             self.push(c as i64);
                             break;
                        }
                        crate::process::scheduler::yield_now();
                   }
                },
                OP_JMP => {
                    let target = self.read_i64(); // Relative or Absolute? Spec says Label -> Addr. So Absolute.
                    self.ip = target as usize;
                },
                OP_JE => {
                    let target = self.read_i64();
                    let b = self.pop();
                    let a = self.pop();
                    if a == b {
                        self.ip = target as usize;
                    }
                },
                OP_CALL => {
                    let target = self.read_i64();
                    self.call_stack.push(self.ip);
                    self.ip = target as usize;
                },
                OP_RET => {
                    if let Some(ret_addr) = self.call_stack.pop() {
                        self.ip = ret_addr;
                    } else {
                        // Stack underflow, exit?
                        self.running = false;
                    }
                },
                OP_KERNEL_OP => {
                    let op_id = self.pop();
                    match op_id {
                        1 => video::clear(),
                        2 => video::put_str("Hello from Nux!\n"),
                        _ => {},
                    }
                },
                OP_EXIT => {
                    self.running = false;
                },
                _ => {
                    // Unknown Op, ignore or crash
                }
            }
        }
        
        video::put_str("NuxVM: Exited.\n");
    }

    fn read_i64(&mut self) -> i64 {
        if self.ip + 8 > self.code.len() { return 0; }
        let val = i64::from_le_bytes(self.code[self.ip..self.ip+8].try_into().unwrap());
        self.ip += 8;
        val
    }

    fn push(&mut self, val: i64) {
        self.stack.push(val);
    }

    fn pop(&mut self) -> i64 {
        self.stack.pop().unwrap_or(0)
    }
}
