use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec::Vec;
use crate::process::task::Task;

/// A simple Mach-like message passing system
pub struct MachMessage {
    pub msg_id: u32,
    pub sender_pid: usize,
    pub data: Vec<u8>,
}

pub struct MachPort {
    pub id: usize,
    pub owner_pid: usize,
    pub messages: Mutex<VecDeque<MachMessage>>,
}

lazy_static::lazy_static! {
    pub static ref MACH_PORTS: Mutex<Vec<Arc<MachPort>>> = Mutex::new(Vec::new());
}

pub fn create_port(pid: usize) -> usize {
    let mut ports = MACH_PORTS.lock();
    let id = ports.len();
    ports.push(Arc::new(MachPort {
        id,
        owner_pid: pid,
        messages: Mutex::new(VecDeque::new()),
    }));
    id
}

pub fn send_message(port_id: usize, msg: MachMessage) -> Result<(), &'static str> {
    let ports = MACH_PORTS.lock();
    if let Some(port) = ports.get(port_id) {
        port.messages.lock().push_back(msg);
        Ok(())
    } else {
        Err("Invalid port")
    }
}

pub fn receive_message(port_id: usize) -> Option<MachMessage> {
    let ports = MACH_PORTS.lock();
    if let Some(port) = ports.get(port_id) {
        port.messages.lock().pop_front()
    } else {
        None
    }
}

pub fn handle_mach_trap(
    sys_num: isize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    _arg4: usize,
    _arg5: usize,
    _arg6: usize,
) -> isize {
    // Negative numbers are Mach Traps
    match sys_num {
        -10 => { // task_create (stub)
            // Normally this would create a Mach task, but we just return an ID
            crate::println!("Mach Trap: task_create called");
            0
        }
        -20 => { // msg_send
            let port = arg1;
            let msg_ptr = arg2 as *const u8;
            let msg_len = arg3;
            
            if !crate::mm::user::validate_user_range(msg_ptr as u64, msg_len) {
                return -1;
            }
            let slice = unsafe { core::slice::from_raw_parts(msg_ptr, msg_len) };
            
            let msg = MachMessage {
                msg_id: 0,
                sender_pid: crate::process::scheduler::get_current_pid(),
                data: slice.to_vec(),
            };
            
            if send_message(port, msg).is_ok() {
                0
            } else {
                -1
            }
        }
        -21 => { // msg_receive
            let port = arg1;
            let msg_ptr = arg2 as *mut u8;
            let max_len = arg3;
            
            if !crate::mm::user::validate_user_range(msg_ptr as u64, max_len) {
                return -1;
            }
            
            if let Some(msg) = receive_message(port) {
                let copy_len = core::cmp::min(max_len, msg.data.len());
                let slice = unsafe { core::slice::from_raw_parts_mut(msg_ptr, copy_len) };
                slice.copy_from_slice(&msg.data[..copy_len]);
                copy_len as isize
            } else {
                -1
            }
        }
        _ => {
            crate::println!("Unknown Mach trap: {}", sys_num);
            -1
        }
    }
}
