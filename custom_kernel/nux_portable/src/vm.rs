use std::vec::Vec;
use std::io::{self, Write, Read};
use std::{thread, time};

const OP_PUSH: u8 = 0x01;
const OP_POP: u8 = 0x02;
const OP_ADD: u8 = 0x10;
const OP_SUB: u8 = 0x11;
const OP_MUL: u8 = 0x12;
const OP_DIV: u8 = 0x13;
const OP_MOD: u8 = 0x14;
const OP_AND: u8 = 0x18;
const OP_OR:  u8 = 0x19; // Bitwise or Logical? Logical usually for flow. VM uses i64 so bitwise is easier to implement but logical needed for `if`. Let's assume Bitwise (& |) and Logical is compiled to jumps. Or we implement logical ops on bools (0/1).
// Let's implement Comparison that pushes 0 or 1.
const OP_EQ: u8 = 0x90;
const OP_NEQ: u8 = 0x91;
const OP_LT: u8 = 0x92;
const OP_GT: u8 = 0x93;
const OP_LTE: u8 = 0x94;
const OP_GTE: u8 = 0x95;
const OP_DRAW_RECT: u8 = 0x20;
const OP_DRAW_IMG: u8 = 0x21; 
const OP_SLEEP: u8 = 0x30;
const OP_DEBUG_PRINT: u8 = 0x50;
const OP_PRINT_CHAR: u8 = 0x51;
const OP_INPUT: u8 = 0x52;
const OP_PRINT_VAL: u8 = 0x53;
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
    virtual_memory: Vec<u8>, // Simple usage for PEEK/POKE
}

impl NuxVm {
    pub fn new(code: Vec<u8>) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            ip: 0,
            code,
            call_stack: Vec::with_capacity(32),
            running: false,
            virtual_memory: vec![0u8; 1024 * 64], // 64KB
        }
    }

    pub fn push(&mut self, val: i64) {
        self.stack.push(val);
    }

    pub fn pop(&mut self) -> i64 {
        self.stack.pop().unwrap_or(0)
    }
    
    fn read_i64(&mut self) -> i64 {
         if self.ip + 8 > self.code.len() { return 0; }
         let bytes = &self.code[self.ip..self.ip+8];
         let val = i64::from_le_bytes(bytes.try_into().unwrap());
         self.ip += 8;
         val
    }

    pub fn run(&mut self) {
        // Verify Header "ANUX" + Version (u16) + Padding(58) = 64 bytes
        if self.code.len() < 64 {
            println!("NuxVM: Invalid Binary (Too Small)");
            return;
        }
        
        if &self.code[0..4] != b"ANUX" {
             println!("NuxVM: Invalid Magic");
             return;
        }

        self.ip = 64; // Start after header
        self.running = true;

        // println!("NuxVM: Started."); 

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
                OP_MUL => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(a.wrapping_mul(b));
                },
                OP_DIV => {
                    let b = self.pop();
                    let a = self.pop();
                    if b == 0 { println!("Runtime Error: Division by Zero"); self.running = false; }
                    else { self.push(a.wrapping_div(b)); }
                },
                OP_MOD => {
                    let b = self.pop();
                    let a = self.pop();
                    if b == 0 { self.push(0); }
                    else { self.push(a.wrapping_rem(b)); }
                },
                OP_EQ => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(if a == b { 1 } else { 0 });
                },
                OP_NEQ => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(if a != b { 1 } else { 0 });
                },
                OP_LT => {
                     let b = self.pop();
                     let a = self.pop();
                     self.push(if a < b { 1 } else { 0 });
                },
                OP_GT => {
                     let b = self.pop();
                     let a = self.pop();
                     self.push(if a > b { 1 } else { 0 });
                },
                OP_LTE => {
                     let b = self.pop();
                     let a = self.pop();
                     self.push(if a <= b { 1 } else { 0 });
                },
                OP_GTE => {
                     let b = self.pop();
                     let a = self.pop();
                     self.push(if a >= b { 1 } else { 0 });
                },
                OP_AND => {
                     let b = self.pop();
                     let a = self.pop();
                     self.push(if a != 0 && b != 0 { 1 } else { 0 });
                },
                OP_OR => {
                     let b = self.pop();
                     let a = self.pop();
                     self.push(if a != 0 || b != 0 { 1 } else { 0 });
                },
                OP_DRAW_RECT => {
                    let _color = self.pop();
                    let _h = self.pop();
                    let _w = self.pop();
                    let _y = self.pop();
                    let _x = self.pop();
                    // Ignored in CLI
                    eprintln!("(Graphic Op: Draw Rect Ignored)");
                },
                OP_SLEEP => {
                    let ms = self.pop();
                    if ms > 0 {
                        thread::sleep(time::Duration::from_millis(ms as u64));
                    }
                },
                OP_DEBUG_PRINT => { 
                     let val = self.pop();
                     println!("DEBUG: {}", val);
                },
                OP_PRINT_CHAR => {
                    let val = self.pop();
                    let c = val as u8 as char;
                    print!("{}", c);
                    io::stdout().flush().unwrap();
                },
                OP_INPUT => {
                   // Read single char from stdin?
                   // Stdin is buffered usually. 
                   // Simple blocking read of one byte
                   let mut buffer = [0u8; 1];
                   if let Ok(_) = io::stdin().read(&mut buffer) {
                       self.push(buffer[0] as i64);
                   } else {
                       self.push(0);
                   }
                },
                OP_PRINT_VAL => {
                    let val = self.pop();
                    print!("{}", val);
                    io::stdout().flush().unwrap();
                },
                OP_JMP => {
                    let target = self.read_i64(); 
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
                        self.running = false;
                    }
                },
                OP_KERNEL_OP => {
                    let op_id = self.pop();
                    match op_id {
                        1 => print!("\x1B[2J\x1B[1;1H"), // Clear Screen (ANSI)
                        2 => print!("Hello from Nux (Portable)!\n"),
                        _ => {},
                    }
                },
                OP_PEEK => {
                    let addr = self.pop();
                    if addr < 0 || addr as usize + 8 > self.virtual_memory.len() {
                        println!("Runtime Error: Memory Access Violation (Read {})", addr);
                        self.running = false;
                    } else {
                        let bytes = &self.virtual_memory[addr as usize .. addr as usize + 8];
                        let val = i64::from_le_bytes(bytes.try_into().unwrap());
                        self.push(val);
                    }
                },
                OP_POKE => {
                    let addr = self.pop();
                    let val = self.pop();
                    if addr < 0 || addr as usize + 8 > self.virtual_memory.len() {
                        println!("Runtime Error: Memory Access Violation (Write {})", addr);
                        self.running = false;
                    } else {
                        let bytes = val.to_le_bytes();
                        for i in 0..8 {
                            self.virtual_memory[addr as usize + i] = bytes[i];
                        }
                    }
                },
                OP_EXIT => {
                    self.running = false;
                },
                _ => {
                    eprintln!("Unknown Opcode: {:02X}", op);
                }
            }
        }
    }
}
