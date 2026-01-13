use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;

pub const AGENT_OP_REGISTER: u64 = 1;
pub const AGENT_OP_QUERY: u64 = 2;
pub const AGENT_OP_INTENT: u64 = 3;

#[derive(Clone)]
pub struct AgentNode {
    pub name: String,
    pub intents: String, // Comma separated for now
    pub binary: String,
    pub trust: u32,
    pub active: bool,
}

// Global Registry
static REGISTRY: Mutex<Vec<AgentNode>> = Mutex::new(Vec::new());

pub fn init() {
    let mut reg = REGISTRY.lock();
    // Register System Agent
    reg.push(AgentNode {
        name: String::from("System"),
        intents: String::from("core,kernel,control"),
        binary: String::from("internal"),
        trust: 2,
        active: true
    });
    crate::drivers::video::put_str("[SEMANTIC] Core Subsystem Initialized.\n");
}

pub fn agent_register(name: &str, intents: &str, binary: &str, trust: u32) -> i64 {
    let mut reg = REGISTRY.lock();
    reg.push(AgentNode {
        name: String::from(name),
        intents: String::from(intents),
        binary: String::from(binary),
        trust,
        active: true
    });
    crate::drivers::video::put_str("[SEMANTIC] Registered Agent: ");
    crate::drivers::video::put_str(name);
    crate::drivers::video::put_str("\n");
    0
}

pub fn agent_query(query: &str) -> Option<String> {
    let reg = REGISTRY.lock();
    for agent in reg.iter() {
        if agent.active && agent.intents.contains(query) {
            return Some(agent.binary.clone());
        }
    }
    None
}

// Syscall Entry Point
pub fn sys_agent_op(op: u64, arg1: u64, arg2: u64) -> u64 {
    match op {
        AGENT_OP_REGISTER => {
            // arg1 = ptr to struct (name_ptr, intents_ptr, binary_ptr, trust)
            // Complex structure passing across syscall boundary is tricky.
            // Simplified: arg1 = ptr to "Name|Intents|Binary|Trust" string?
            // Or just multiple syscalls?
            // For now, let's assume arg1 is a ptr to a C-struct-like packed data.
            // But we need to fetch multiple pointers.
            // Mock: Just return 0. Real impl needs userspace memory access.
            0 
        },
        AGENT_OP_QUERY => {
            // arg1 = ptr to query string, arg2 = ptr to output buffer
            // 1. Fetch Query
            // We need to know length. Assume C-string (null terminated)?
            // Rust kernel helper needed: fetch_string_from_user(ptr)
            
            // This is complex without a robust userspace accessor helper.
            // We'll stub with a fixed buffer read for now.
            let query_ptr = arg1 as *const u8;
            let out_ptr = arg2 as *mut u8;
            
            // Unsafe read 64 bytes for query
            let mut query_buf = [0u8; 64];
            if crate::mm::user::copy_from_user(query_ptr, &mut query_buf).is_err() {
                 return u64::MAX;
            }
            let query_str = match core::str::from_utf8(&query_buf) {
                Ok(s) => s.trim_matches(char::from(0)),
                Err(_) => return u64::MAX,
            };
            
            if let Some(binary) = agent_query(query_str) {
                // Copy result to user
                if crate::mm::user::copy_to_user(out_ptr, binary.as_bytes()).is_ok() {
                    0
                } else {
                     u64::MAX
                }
            } else {
                 u64::MAX 
            }
        },
        AGENT_OP_INTENT => {
             // arg1 = ptr to intent string
             let ptr = arg1 as *const u8;
             let mut buf = [0u8; 64];
              if crate::mm::user::copy_from_user(ptr, &mut buf).is_err() {
                 return u64::MAX;
            }
             if let Ok(s) = core::str::from_utf8(&buf) {
                 crate::drivers::video::put_str("[SEMANTIC] INTENT: ");
                 crate::drivers::video::put_str(s);
                 crate::drivers::video::put_str("\n");
             }
             0
        },
        _ => u64::MAX
    }
}
