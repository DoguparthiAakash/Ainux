use alloc::vec::Vec;
use spin::Mutex;
use alloc::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum IpcPayload {
    Inline([u64; 8]),         // Standard small payload
    Memory(usize, usize),     // (Address, Size) - for passing memory handles/descriptors
    Capability(usize),        // Passing a capability ID
}

#[derive(Debug, Clone)]
pub struct Message {
    pub sender_pid: usize,
    pub msg_type: u32,
    pub payload: IpcPayload,
}

pub struct Port {
    id: usize,
    queue: Mutex<VecDeque<Message>>,
    waiting_tasks: Mutex<VecDeque<usize>>,
}

pub static PORTS: Mutex<Vec<Option<Port>>> = Mutex::new(Vec::new());

// Global well-known port for the POSIX subsystem
pub static POSIX_SUBSYSTEM_PORT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(usize::MAX);

pub fn create_port() -> usize {
    let mut ports = PORTS.lock();
    let id = ports.len();
    ports.push(Some(Port {
        id,
        queue: Mutex::new(VecDeque::new()),
        waiting_tasks: Mutex::new(VecDeque::new()),
    }));
    id
}

pub fn send(port_id: usize, msg: Message) -> bool {
    let ports = PORTS.lock();
    if let Some(Some(port)) = ports.get(port_id) {
        port.queue.lock().push_back(msg);
        
        // Wake up ONE waiting task
        if let Some(pid) = port.waiting_tasks.lock().pop_front() {
            crate::process::scheduler::wake_task(pid);
        }
        return true;
    }
    false
}

pub fn receive(port_id: usize, non_blocking: bool) -> Option<Message> {
    loop {
        let ports = PORTS.lock();
        if let Some(Some(port)) = ports.get(port_id) {
            let mut queue = port.queue.lock();
            if let Some(msg) = queue.pop_front() {
                return Some(msg);
            } else {
                if non_blocking {
                    return None;
                }
                // Empty, Block me
                let current_pid = crate::process::scheduler::get_current_pid();
                port.waiting_tasks.lock().push_back(current_pid);
                drop(queue);
                drop(ports); // Drop locks before switching!
                
                crate::process::scheduler::block_current_task();
                // When we return here, we were woken up. Loop again to check queue.
            }
        } else {
            return None; // Port Invalid
        }
    }
}
