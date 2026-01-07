use std::vec::Vec;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
#[cfg(feature = "gui")]
use minifb::{Window, WindowOptions, Scale};
use std::{thread, time};
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::UnsafeCell;

// ... constants ...
const OP_PUSH: u8 = 0x01;
const OP_POP: u8 = 0x02;
// ... arithmetic ...
const OP_ADD: u8 = 0x10;
const OP_SUB: u8 = 0x11;
const OP_MUL: u8 = 0x12;
const OP_DIV: u8 = 0x13;
const OP_MOD: u8 = 0x14;
const OP_AND: u8 = 0x18;
const OP_OR:  u8 = 0x19; 
const OP_EQ: u8 = 0x90;
const OP_NEQ: u8 = 0x91;
const OP_LT: u8 = 0x92;
const OP_GT: u8 = 0x93;
const OP_LTE: u8 = 0x94;
const OP_GTE: u8 = 0x95;

const OP_DRAW_RECT: u8 = 0x20;
const OP_DRAW_IMG: u8 = 0x21; // Unused
const OP_SLEEP: u8 = 0x30;

const OP_DEBUG_PRINT: u8 = 0x50;
const OP_PRINT_CHAR: u8 = 0x51;
const OP_INPUT: u8 = 0x52;
const OP_PRINT_VAL: u8 = 0x53; // Prints i64
const OP_PRINT_FLOAT: u8 = 0x54; // Prints f64

// Float Ops
const OP_FADD: u8 = 0x1A;
const OP_FSUB: u8 = 0x1B;
const OP_FMUL: u8 = 0x1C;
const OP_FDIV: u8 = 0x1D;
const OP_ITOF: u8 = 0x1E; // Int to Float
const OP_FTOI: u8 = 0x1F; // Float to Int

const OP_PEEK: u8 = 0x40;
const OP_POKE: u8 = 0x41;
const OP_PEEK8: u8 = 0x42;
const OP_POKE8: u8 = 0x43;
// 42/43 PEEK8/POKE8 unused

const OP_JMP: u8 = 0x60;
const OP_JE: u8 = 0x61;
// const OP_JNE: u8 = 0x62; // Future?

const OP_CALL: u8 = 0x70;
const OP_RET: u8 = 0x71;
const OP_SPAWN: u8 = 0x72; // NEW: Spawn Thread
const OP_LOCK: u8 = 0x73;  // NEW: Acquire Lock (Simple Global Lock or ID?)
const OP_UNLOCK: u8 = 0x74; // NEW: Release Lock

const OP_KERNEL_OP: u8 = 0x80;

// Vision/Image Opcodes
const OP_IMG_CREATE: u8 = 0x80; // Reusing KERNEL_OP range or distinct? Let's use 0x80-0x85 overriding KERNEL_OP?
// Wait, KERNEL_OP is 0x80. Let's move Vision to 0xA0.
// Actually KERNEL_OP is just one.
const OP_IMG_ALLOC: u8 = 0xA0;
const OP_IMG_FREE: u8 = 0xA1;
const OP_CAM_CAPTURE: u8 = 0xA2;
const OP_IMG_GET: u8 = 0xA3;
const OP_IMG_SET: u8 = 0xA4;
const OP_IMG_FILTER: u8 = 0xA5;

const OP_EXIT: u8 = 0xFF; // Keep at end

#[derive(Debug, Clone)]
struct ImageBuffer {
    width: usize,
    height: usize,
    data: Vec<u8>, // Grayscale 8-bit
}

impl ImageBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self { width, height, data: vec![0; width * height] }
    }
}

// Simple SpinLock Implementation for Kernel Safety
pub struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin hint could go here (std::hint::spin_loop())
            // but might not be available in all portable contexts.
             std::thread::yield_now(); // Be nice to scheduler
        }
        SpinLockGuard { lock: self }
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> std::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> std::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
    }
}


// Shared State for all threads
struct SharedState {
    memory: SpinLock<Vec<u8>>, // Global Virtual Memory (Heap/Globals)
    // We could add a mutex map for fine-grained locks later.
    // For now, implicit global lock or use atomic memory ops?
    // User requested "thread safety". Mutex around memory is safe *access*.
    // But logic race needs explicit locks.
    locks: std::collections::HashMap<u64, Arc<SpinLock<()>>>, 
    // Actually simpler: One Big Lock for critical sections if requested?
    // Or users provide lock ID.
}

pub struct NuxVm {
    // Thread-Local State
    stack: Vec<i64>,
    pc: usize, // Renamed from ip
    call_stack: Vec<usize>,
    running: bool,
    
    // Vision System State
    images: Vec<Option<ImageBuffer>>,
    #[cfg(feature = "gui")]
    window: Option<Window>,
    
    // Shared State
    program: Arc<Vec<u8>>, // Renamed from code
    shared: Arc<SharedState>,
}

impl NuxVm {
    pub fn new(program: Vec<u8>) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            pc: 64, // Skip Header
            call_stack: Vec::with_capacity(32),
            running: false,
            images: Vec::new(),
            #[cfg(feature = "gui")]
            window: None,
            program: Arc::new(program),
            shared: Arc::new(SharedState {
                memory: SpinLock::new(vec![0; 1024 * 1024]), // 1MB Shared Heap
                locks: std::collections::HashMap::new(),
            }),
        }
    }
    
    // New internal constructor for cloning shared state
    fn fork(&self, start_ip: usize) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            pc: start_ip, // Renamed from ip
            call_stack: Vec::with_capacity(32),
            running: true,
            program: self.program.clone(), // Renamed from code
            shared: self.shared.clone(),
            images: Vec::new(), // Each thread gets its own image handles
            #[cfg(feature = "gui")]
            window: None,
        }
    }

    pub fn push(&mut self, val: i64) {
        if self.stack.len() >= 1024 {
            println!("Runtime Error: Stack Overflow");
            self.running = false;
            return;
        }
        self.stack.push(val);
    }

    pub fn pop(&mut self) -> i64 {
        if self.stack.is_empty() {
             println!("Runtime Error: Stack Underflow");
             self.running = false;
             return 0;
        }
        self.stack.pop().unwrap_or(0)
    }
    
    fn read_i64_code(&mut self) -> i64 {
         if self.pc + 8 > self.program.len() { return 0; }
         let bytes = &self.program[self.pc..self.pc+8];
         let val = i64::from_le_bytes(bytes.try_into().unwrap());
         self.pc += 8;
         val
    }

    pub fn run(&mut self) {
        // Only check header if starting from 0 (main thread)
        // Sub-threads start at specific function.
        if self.pc == 0 {
             if self.program.len() < 64 || &self.program[0..4] != b"ANUX" {
                 println!("NuxVM: Invalid Binary");
                 return;
             }
             self.pc = 64; 
        }
        
        self.running = true;

        while self.running && self.pc < self.program.len() {
            let op = self.program[self.pc];
            if self.stack.len() > 10 {
                 println!("PC: {}, OP: {:02X}, Stack: {}", self.pc, op, self.stack.len());
            }
            self.pc += 1;

            match op {
                OP_PUSH => {
                    let val = self.read_i64_code();
                    self.push(val);
                },
                OP_POP => { self.pop(); },
                OP_ADD => { let b = self.pop(); let a = self.pop(); self.push(a.wrapping_add(b)); },
                OP_SUB => { let b = self.pop(); let a = self.pop(); self.push(a.wrapping_sub(b)); },
                OP_MUL => { let b = self.pop(); let a = self.pop(); self.push(a.wrapping_mul(b)); },
                OP_DIV => { 
                    let b = self.pop(); let a = self.pop(); 
                    if b == 0 { println!("Runtime Error: DivZero"); self.running = false; }
                    else { self.push(a.wrapping_div(b)); }
                },
                OP_MOD => { let b = self.pop(); let a = self.pop(); if b!=0 { self.push(a%b); } else { self.push(0); } },
                
                // Float Ops
                OP_FADD => { 
                    let b = f64::from_bits(self.pop() as u64); 
                    let a = f64::from_bits(self.pop() as u64); 
                    self.push((a + b).to_bits() as i64); 
                },
                OP_FSUB => { 
                    let b = f64::from_bits(self.pop() as u64); 
                    let a = f64::from_bits(self.pop() as u64); 
                    self.push((a - b).to_bits() as i64); 
                },
                OP_FMUL => { 
                    let b = f64::from_bits(self.pop() as u64); 
                    let a = f64::from_bits(self.pop() as u64); 
                    self.push((a * b).to_bits() as i64); 
                },
                OP_FDIV => { 
                    let b = f64::from_bits(self.pop() as u64); 
                    let a = f64::from_bits(self.pop() as u64); 
                    self.push((a / b).to_bits() as i64); 
                },
                OP_ITOF => {
                    let a = self.pop();
                    self.push((a as f64).to_bits() as i64);
                },
                OP_FTOI => {
                    let a = f64::from_bits(self.pop() as u64);
                    self.push(a as i64);
                },
                OP_PRINT_FLOAT => {
                    let val = f64::from_bits(self.pop() as u64);
                    print!("{}", val);
                    io::stdout().flush().unwrap();
                },
                
                OP_EQ => { let b = self.pop(); let a = self.pop(); self.push(if a == b {1} else {0}); },
                OP_NEQ => { let b = self.pop(); let a = self.pop(); self.push(if a != b {1} else {0}); },
                OP_LT => { let b = self.pop(); let a = self.pop(); self.push(if a < b {1} else {0}); },
                OP_GT => { let b = self.pop(); let a = self.pop(); self.push(if a > b {1} else {0}); },
                OP_LTE => { let b = self.pop(); let a = self.pop(); self.push(if a <= b {1} else {0}); },
                OP_GTE => { let b = self.pop(); let a = self.pop(); self.push(if a >= b {1} else {0}); },
                
                OP_AND => { let b = self.pop(); let a = self.pop(); self.push(if a!=0 && b!=0 {1} else {0}); },
                OP_OR => { let b = self.pop(); let a = self.pop(); self.push(if a!=0 || b!=0 {1} else {0}); },

                OP_DRAW_IMG => {
                    // Draw Image to Window (and fallback/debug to Console)
                    let handle = self.pop() as usize;
                    if handle < self.images.len() {
                        if let Some(img) = &self.images[handle] {
                             #[cfg(feature = "gui")] 
                             {
                                 let width = img.width;
                                 let height = img.height;
                                 
                                 let buffer: Vec<u32> = img.data.iter().map(|&v| {
                                     let val = v as u32;
                                     0xFF000000 | (val << 16) | (val << 8) | val
                                 }).collect();

                                 if self.window.is_none() {
                                     let win_result = Window::new(
                                         "Nux Vision Output",
                                         width,
                                         height,
                                         WindowOptions {
                                            scale: Scale::X4,
                                            ..WindowOptions::default()
                                         }
                                     );
                                     
                                     match win_result {
                                         Ok(mut win) => {
                                             win.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
                                             self.window = Some(win);
                                         },
                                         Err(e) => {
                                             println!("Warning: Could not create window: {}", e);
                                         }
                                     }
                                 }

                                 if let Some(win) = &mut self.window {
                                     if let Err(e) = win.update_with_buffer(&buffer, width, height) {
                                         println!("Window Update Error: {}", e);
                                     }
                                 }
                             }
                             
                             #[cfg(not(feature = "gui"))]
                             {
                                 println!("--- Image Dump ({}x{}) ---", img.width, img.height);
                                 let chars = b" .:-=+*#%@";
                                 for y in 0..img.height {
                                     for x in 0..img.width {
                                         let val = img.data[y * img.width + x] as usize;
                                         let char_idx = (val * (chars.len() - 1)) / 255;
                                         print!("{}", chars[char_idx] as char);
                                     }
                                     print!("\n");
                                 }
                                 println!("-------------------------");
                             }
                        }
                    }
                    self.push(0); // Void Return
                },
                OP_SLEEP => {
                    let ms = self.pop();
                    if ms > 0 { thread::sleep(time::Duration::from_millis(ms as u64)); }
                },
                
                // --- Vision Ops ---
                OP_IMG_ALLOC => {
                    let h = self.pop() as usize;
                    let w = self.pop() as usize;
                    let img = ImageBuffer::new(w, h);
                    
                    // Find free slot or push
                    let mut slot = None;
                    for (i, opt) in self.images.iter_mut().enumerate() {
                        if opt.is_none() {
                            *opt = Some(img.clone()); // optimization: move
                            slot = Some(i);
                            break;
                        }
                    }
                    if slot.is_none() {
                        self.images.push(Some(img));
                        slot = Some(self.images.len() - 1);
                    }
                    self.push(slot.unwrap() as i64);
                },
                OP_IMG_FREE => {
                    let handle = self.pop() as usize;
                    if handle < self.images.len() {
                        self.images[handle] = None;
                    }
                    self.push(0); // Void Return
                },
                OP_CAM_CAPTURE => {
                     let handle = self.pop() as usize;
                     if handle < self.images.len() {
                        if let Some(img) = &mut self.images[handle] {
                            // Simulate Capture: Moving Gradient
                            // We use time based or just linear invalidation
                             let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as usize;
                             for y in 0..img.height {
                                 for x in 0..img.width {
                                     // Create some pattern
                                     let v = ((x + y + (seed / 100)) % 255) as u8;
                                     img.data[y * img.width + x] = v;
                                 }
                             }
                        }
                     }
                     self.push(0); // Void Return
                },
                OP_IMG_GET => {
                    let y = self.pop() as usize;
                    let x = self.pop() as usize;
                    let handle = self.pop() as usize;
                    let mut val = 0;
                    if handle < self.images.len() {
                        if let Some(img) = &self.images[handle] {
                            if x < img.width && y < img.height {
                                val = img.data[y * img.width + x] as i64;
                            }
                        }
                    }
                    self.push(val);
                },
                OP_IMG_SET => {
                    let val = self.pop() as u8;
                    let y = self.pop() as usize;
                    let x = self.pop() as usize;
                    let handle = self.pop() as usize;
                     if handle < self.images.len() {
                        if let Some(img) = &mut self.images[handle] {
                            if x < img.width && y < img.height {
                                img.data[y * img.width + x] = val;
                            }
                        }
                     }
                     self.push(0); // Void Return
                },
                OP_IMG_FILTER => {
                    let filter_id = self.pop();
                    let handle = self.pop() as usize;
                     if handle < self.images.len() {
                         // Clone to avoid borrow issues if we did sophisticated processing
                         // For now simple in-place
                         if let Some(img) = &mut self.images[handle] {
                             match filter_id {
                                 1 => { // Threshold
                                     for p in img.data.iter_mut() {
                                         *p = if *p > 128 { 255 } else { 0 };
                                     }
                                 },
                                 2 => { // Invert
                                     for p in img.data.iter_mut() {
                                         *p = 255 - *p;
                                     }
                                 },
                                 _ => {}
                             }
                         }
                     }
                     self.push(0); // Void Return
                },
                OP_DEBUG_PRINT => { let val = self.pop(); println!("[Thread {:?}] DEBUG: {}", thread::current().id(), val); },
                OP_PRINT_CHAR => { 
                    let val = self.pop(); print!("{}", val as u8 as char); io::stdout().flush().unwrap(); 
                },
                OP_PRINT_VAL => { let val = self.pop(); print!("{}", val); io::stdout().flush().unwrap(); },
                
                OP_INPUT => {
                   let mut buffer = String::new();
                   if let Ok(_) = io::stdin().read_line(&mut buffer) {
                       let val = buffer.trim().parse::<i64>().unwrap_or(0);
                       self.push(val); 
                   } else { 
                       self.push(0); 
                   }
                },

                OP_JMP => { let t = self.read_i64_code(); self.pc = t as usize; },
                OP_JE => { 
                    let t = self.read_i64_code(); 
                    let b = self.pop(); let a = self.pop(); 
                    if a == b { self.pc = t as usize; }
                },
                
                OP_CALL => {
                    let t = self.read_i64_code();
                    if self.call_stack.len() >= 256 {
                        println!("Runtime Error: Call Stack Overflow (Recursion too deep)");
                        self.running = false;
                    } else {
                        self.call_stack.push(self.pc);
                        self.pc = t as usize;
                    }
                },
                OP_RET => {
                    if let Some(ret) = self.call_stack.pop() { self.pc = ret; }
                    else { self.running = false; }
                },
                
                // --- THREADING_OPS ---
                OP_SPAWN => {
                    let target = self.read_i64_code(); // Function address
                    // Fork a VM instance
                    let mut child_vm = self.fork(target as usize);
                    
                    // Spawn OS Thread
                    thread::spawn(move || {
                        child_vm.run();
                    });
                    // println!("DEBUG: Spawning thread at {}", target);
                },
                // Locking ops (TODO: Implement proper ID-based locks if needed)
                OP_LOCK => { /* Placeholder */ },
                OP_UNLOCK => { /* Placeholder */ },

                OP_KERNEL_OP => {
                    let op_id = self.pop();
                    match op_id {
                        1 => print!("\x1B[2J\x1B[1;1H"),
                        2 => println!("NuxVM Multi-Threaded v0.4"),
                        _ => {},
                    }
                },
                
                // Memory Ops (Thread-Safe via Mutex)
                OP_PEEK => {
                    let addr = self.pop();
                    let shared = self.shared.clone(); 
                    let val_opt = {
                        let state = shared.memory.lock();
                        if addr < 0 || addr as usize + 8 > state.len() {
                             None
                        } else {
                             let bytes = &state[addr as usize .. addr as usize + 8];
                             Some(i64::from_le_bytes(bytes.try_into().unwrap()))
                        }
                    };
                    
                    if let Some(val) = val_opt {
                        self.push(val);
                    } else {
                        println!("Runtime Error: Segfault Read at {}", addr);
                         self.running = false;
                    }
                },
                OP_POKE => {
                    let val = self.pop();
                    let addr = self.pop();
                    let shared = self.shared.clone();
                    
                     let ok = {
                        let mut state = shared.memory.lock();
                        if addr < 0 || addr as usize + 8 > state.len() {
                             false
                        } else {
                             let bytes = val.to_le_bytes();
                             for i in 0..8 {
                                 state[addr as usize + i] = bytes[i];
                             }
                             true
                        }
                    };
                    
                     if !ok {
                        println!("Runtime Error: Segfault Write at {}", addr);
                        self.running = false;
                    }
                },
                OP_PEEK8 => {
                    let addr = self.pop();
                    let shared = self.shared.clone();
                    let val_opt = {
                        let state = shared.memory.lock();
                         if addr < 0 || addr as usize + 1 > state.len() { None }
                         else { Some(state[addr as usize] as i64) }
                    };
                    if let Some(v) = val_opt { self.push(v); }
                    else { println!("Runtime Error: Segfault Read8 at {}", addr); self.running = false; }
                },
                OP_POKE8 => {
                    let val = self.pop();
                    let addr = self.pop();
                    let shared = self.shared.clone();
                    let ok = {
                        let mut state = shared.memory.lock();
                        if addr < 0 || addr as usize + 1 > state.len() { false }
                        else { state[addr as usize] = val as u8; true }
                    };
                    if !ok { println!("Runtime Error: Segfault Write8 at {}", addr); self.running = false; }
                },
                
                OP_EXIT => { self.running = false; },
                _ => { eprintln!("Unknown Opcode: {:02X}", op); }
            }
        }
    }
}

