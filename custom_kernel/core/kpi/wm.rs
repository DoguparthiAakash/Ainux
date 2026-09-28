// Ainux Window Manager (GUI Server)
// Built natively on the AAA framework using Capability IPC and Zero-Copy Shared Memory

use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec::Vec;
use crate::kpi::aaa::{AinuxAppNode, Capability, IpcMessage, PERM_IPC, PERM_SHMEM, register_app};

pub const WM_PID: usize = 100; // Reserved PID for the Window Manager

/// Structure of the Window Manager Service
pub struct WindowManager {
    node: Arc<AinuxAppNode>,
    windows: Mutex<Vec<Window>>,
}

/// A graphical window mapped to a client's shared memory
pub struct Window {
    pub client_pid: usize,
    pub width: u32,
    pub height: u32,
    pub shmem_ptr: u64, // Zero-copy pointer to the client's framebuffer
}

lazy_static::lazy_static! {
    pub static ref WM_SERVER: WindowManager = WindowManager::new();
}

impl WindowManager {
    fn new() -> Self {
        let node = register_app(WM_PID);
        Self {
            node,
            windows: Mutex::new(Vec::new()),
        }
    }

    /// Primary daemon loop for the Window Manager
    pub fn process_events(&self) {
        while let Some(msg) = self.node.receive_message() {
            self.handle_ipc(msg);
        }
    }

    fn handle_ipc(&self, msg: IpcMessage) {
        if let Some(cap) = msg.capability {
            // Check if the client has permission to map shared memory to the screen
            if (cap.permissions & PERM_SHMEM) != 0 {
                if let Some(ptr) = msg.shared_mem_ptr {
                    // Create a new window from the zero-copy shared memory pointer
                    crate::println!("[WM] Accepted window framebuffer from Client PID: {} at {:#x}", msg.sender_pid, ptr);
                    
                    {
                        let mut wins = self.windows.lock();
                        wins.push(Window {
                            client_pid: msg.sender_pid,
                            width: 800, // Hardcoded for demo
                            height: 600,
                            shmem_ptr: ptr,
                        });
                    }
                    
                    // Trigger a compositor redraw
                    let _ = crate::drivers::serial::SerialPort::new(0x3F8).write_byte(b'C');
                    self.composite();
                    let _ = crate::drivers::serial::SerialPort::new(0x3F8).write_byte(b'D');
                }
            } else {
                crate::println!("[WM] Rejected request from PID {}: Missing PERM_SHMEM capability", msg.sender_pid);
            }
        }
    }

    fn composite(&self) {
        let _ = crate::drivers::serial::SerialPort::new(0x3F8).write_byte(b'L');
        let wins = self.windows.lock();
        let _ = crate::drivers::serial::SerialPort::new(0x3F8).write_byte(b'P');
        crate::println!("[WM] Compositing {} windows to physical framebuffer...", wins.len());
        // In a real implementation, this reads from window.shmem_ptr and writes to VRAM
    }
}

/// A mock client application attempting to use the GUI
pub fn run_mock_client() {
    let client = register_app(101);
    
    // The kernel (or init system) grants the client a capability to talk to the WM and use SHMEM
    let wm_cap = Capability {
        id: 1,
        permissions: PERM_IPC | PERM_SHMEM,
        owner_pid: WM_PID,
    };
    let _ = client.grant(wm_cap.clone());

    crate::println!("[Client 101] Sending zero-copy framebuffer to Window Manager via AAA IPC...");
    
    // Simulate a 4MB framebuffer allocated in shared memory
    let fake_shmem_addr: u64 = 0x8000_0000;
    
    let msg = IpcMessage {
        sender_pid: client.pid,
        capability: Some(wm_cap),
        data: Vec::new(),
        shared_mem_ptr: Some(fake_shmem_addr),
    };

    let wm_node = crate::kpi::aaa::APP_NODES.lock().get(&WM_PID).unwrap().clone();
    
    if let Err(e) = client.send_message(&wm_node, msg) {
        crate::println!("[Client 101] Failed to send IPC: {}", e);
    }
}

pub fn init() {
    crate::println!("Starting Ainux AAA Window Manager Server...");
    
    // Force initialization of the lazy_static WM_SERVER so it registers WM_PID
    lazy_static::initialize(&WM_SERVER);
    
    // Spin up the mock client to send a frame
    run_mock_client();
    
    // Process the messages sent by clients
    WM_SERVER.process_events();
}
