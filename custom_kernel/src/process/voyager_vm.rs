use alloc::vec::Vec;
use crate::drivers::video;

pub struct VoyagerVM {
    stack: Vec<i64>,
    pc: usize,
    code: Vec<u8>,
    // Quantum state: 8 qubits (simple amplitude simulation)
    // amplitudes[i] = (real, imag)
    q_real: [i32; 256], // 2^8 states
    is_quantum: bool,
}

impl VoyagerVM {
    pub fn new(code: Vec<u8>) -> Self {
        Self {
            stack: Vec::with_capacity(32),
            pc: 64, // Skip header
            code,
            q_real: [0; 256],
            is_quantum: false,
        }
    }

    pub fn run(&mut self) {
        if self.code.len() < 64 { return; }
        
        let magic = &self.code[0..4];
        if magic == b"QNUX" {
            self.is_quantum = true;
            self.q_real[0] = 1000; // |0...0> state
        }

        while self.pc < self.code.len() {
            let op = self.code[self.pc];
            self.pc += 1;

            match op {
                0x01 => { // PUSH imm64
                    let val = i64::from_le_bytes(self.code[self.pc..self.pc+8].try_into().unwrap());
                    self.stack.push(val);
                    self.pc += 8;
                },
                0x02 => { // POP
                    self.stack.pop();
                },
                0x10 => { // ADD
                    let b = self.stack.pop().unwrap_or(0);
                    let a = self.stack.pop().unwrap_or(0);
                    self.stack.push(a + b);
                },
                0x11 => { // SUB
                    let b = self.stack.pop().unwrap_or(0);
                    let a = self.stack.pop().unwrap_or(0);
                    self.stack.push(a - b);
                },
                0x20 => { // PRINT
                    let c = self.stack.pop().unwrap_or(32) as u8 as char;
                    video::put_char(c);
                },
                0x21 => { // RECT (x, y, w, h)
                    let h = self.stack.pop().unwrap_or(0) as usize;
                    let w = self.stack.pop().unwrap_or(0) as usize;
                    let y = self.stack.pop().unwrap_or(0) as usize;
                    let x = self.stack.pop().unwrap_or(0) as usize;
                    // Assuming a color fixed at White for now or popped too
                    let color = 0xFFFFFFFF;
                    for dy in 0..h {
                        for dx in 0..w {
                            crate::drivers::video::draw_pixel((x + dx) as i64, (y + dy) as i64, color);
                        }
                    }
                },
                0x30 => { // JMP
                    let offset = i32::from_le_bytes(self.code[self.pc..self.pc+4].try_into().unwrap());
                    // This is crude, normally JMP is to a label but here we use relative offset
                    let new_pc = (self.pc as i32 + offset) as usize;
                    self.pc = new_pc;
                },
                0x41 => { // Q_HAD (qid) - Simplified Hadamard
                    if self.is_quantum {
                        let qid = self.code[self.pc] as usize;
                        self.pc += 1;
                        video::put_str(&format!("[System] Applied State Transformation to Qubit {}\n", qid));
                        // In a real sim we'd rotate amplitudes. Here we just flag superpos.
                    }
                },
                0x43 => { // Q_MEAS (qid)
                    if self.is_quantum {
                        let qid = self.code[self.pc] as usize;
                        self.pc += 1;
                        // Random-ish measurement based on ticks
                        let ticks = crate::process::scheduler::get_ticks();
                        let res = (ticks % 2) as i64;
                        self.stack.push(res);
                        video::put_str(&format!("[System] Measured State Qubit {}: {}\n", qid, res));
                    }
                },
                0xFF => break, // EXIT
                _ => {
                    video::put_str(&format!("VM Error: Unknown op 0x{:02X}\n", op));
                    break;
                }
            }
        }
    }
}

pub fn run_voyager_file(path: &str) {
    let root = crate::fs::vfs::ROOT.lock();
    if let Some(r) = root.as_ref() {
        if let Ok(inode) = r.lookup(path) {
            if let Ok(handle) = inode.open(0) {
                 let mut buf = alloc::vec![0u8; 16384];
                 if let Ok(n) = handle.read(&mut buf, 0) {
                     buf.truncate(n);
                     drop(root); // Unlock before running
                     let mut vm = VoyagerVM::new(buf);
                     vm.run();
                     return;
                 }
            }
        }
    }
    video::put_str("VM: Could not load Sovereign binary.\n");
}
