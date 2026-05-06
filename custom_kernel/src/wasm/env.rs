use wasmi::{Linker, Error, Caller, Extern};
use core::fmt::Write;
use crate::wasm::{Object, Rights, WasmProcessState, HandleEntry, GLOBAL_EPOCH, ChannelMessage, REGISTRY};
use core::sync::atomic::Ordering;

// Phase 7 Opcodes
const OP_WRITE: u32 = 1;
const OP_EXIT: u32 = 2;
const OP_OPEN: u32 = 3;
const OP_READ: u32 = 4;
const OP_CLOSE: u32 = 5;
const OP_PUSH_HDL: u32 = 6;
const OP_PULL_HDL: u32 = 7;
const OP_REGISTER: u32 = 8;
const OP_LOOKUP: u32 = 9;
const OP_SPAWN: u32 = 10; // Step 8.3: procd-only syscall

pub fn add_to_linker(linker: &mut Linker<WasmProcessState>) -> Result<(), Error> {
    linker.func_wrap("env", "cap_call", |mut caller: Caller<'_, WasmProcessState>, handle: u32, op: u32, ptr: u32, len: u32| -> i32 {
        let index = (handle & 0xFFFF) as usize;
        let gen = (handle >> 16) & 0xFFFF;

        // 1. Handle Lookup
        let entry = match caller.data().handles.get(index) {
            Some(Some(e)) if e.generation == gen as u32 => e.clone(),
            _ => return -1, // Invalid Handle
        };

        // 2. Epoch Validation (Layered)
        if entry.cap.global_epoch != GLOBAL_EPOCH.load(Ordering::SeqCst) {
            return -11; // Global Revocation
        }
        if entry.cap.resource_epoch != crate::wasm::get_resource_epoch(entry.cap.resource_id) {
            return -11; // Resource-Specific Revocation
        }

        let cap = entry.cap;

        match op {
            OP_WRITE => {
                if (cap.rights.0 & Rights::WRITE) == 0 { return -2; }
                match cap.obj {
                    Object::Console => {
                        let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                        let data = memory.data(&caller);
                        let end = (ptr + len) as usize;
                        if end > data.len() { return -3; }
                        let slice = &data[ptr as usize..end];
                        if let Ok(s) = core::str::from_utf8(slice) {
                            let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                            let _ = write!(serial, "[CapWrite:{}] {}\n", index, s);
                            return len as i32;
                        }
                        return -4;
                    }
                    Object::File(ref h) => {
                        let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                        let data = memory.data(&caller);
                        let end = (ptr + len) as usize;
                        if end > data.len() { return -3; }
                        let slice = &data[ptr as usize..end];
                        match h.write(slice, 0) {
                            Ok(n) => return n as i32,
                            Err(_) => return -5,
                        }
                    }
                    _ => return -6,
                }
            }
            OP_OPEN => {
                if (cap.rights.0 & Rights::LOOKUP) == 0 { return -2; }
                if let Object::Directory(ref dir) = cap.obj {
                    let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                    let data = memory.data(&caller);
                    let end = (ptr + len) as usize;
                    if end > data.len() { return -3; }
                    let path_slice = &data[ptr as usize..end];
                    if let Ok(path) = core::str::from_utf8(path_slice) {
                        if let Ok(inode) = dir.lookup(path) {
                            // Step 7.2: Structural Confinement Check
                            if !crate::fs::vfs::is_descendant_of(dir.clone(), inode.clone()) {
                                return -18; // Confinement Violation
                            }
                            
                            if let Ok(file_handle) = inode.open(0) {
                                let new_cap = crate::wasm::Capability {
                                    obj: Object::File(file_handle),
                                    rights: Rights(Rights::READ | Rights::WRITE | Rights::TRANSFER),
                                    global_epoch: GLOBAL_EPOCH.load(Ordering::SeqCst),
                                    resource_id: 2, // Storage
                                    resource_epoch: crate::wasm::get_resource_epoch(2),
                                };
                                return add_cap_to_table(&mut caller, new_cap);
                            }
                        }
                    }
                    return -7;
                }
                -8
            }
            OP_READ => {
                if (cap.rights.0 & Rights::READ) == 0 { return -2; }
                if let Object::File(ref h) = cap.obj {
                    let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                    let data = memory.data_mut(&mut caller);
                    let end = (ptr + len) as usize;
                    if end > data.len() { return -3; }
                    let slice = &mut data[ptr as usize..end];
                    match h.read(slice, 0) {
                        Ok(n) => return n as i32,
                        Err(_) => return -5,
                    }
                }
                -9
            }
            OP_PUSH_HDL => {
                if let Object::Channel(ref chan) = cap.obj {
                    // Step 8.2: IPC Quota & Backpressure
                    if chan.queue.lock().len() >= chan.max_size {
                        return -19; // Quota Exhausted / Backpressure
                    }

                    let target_idx = (ptr & 0xFFFF) as usize;
                    let target_gen = (ptr >> 16) & 0xFFFF;
                    let target_entry = match caller.data().handles.get(target_idx) {
                        Some(Some(e)) if e.generation == target_gen as u32 => e.clone(),
                        _ => return -1,
                    };
                    if (target_entry.cap.rights.0 & Rights::TRANSFER) == 0 { return -13; }
                    let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                    let data_buf = memory.data(&caller);
                    let data_ptr = len as usize;
                    if data_ptr + 32 > data_buf.len() { return -3; }
                    let mut payload = [0u8; 32];
                    payload.copy_from_slice(&data_buf[data_ptr..data_ptr+32]);
                    chan.queue.lock().push_back(ChannelMessage { data: payload, cap: Some(target_entry.cap) });
                    return 0;
                }
                return -14;
            }
            OP_PULL_HDL => {
                if let Object::Channel(ref chan) = cap.obj {
                    if let Some(msg) = chan.queue.lock().pop_front() {
                        let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                        let data_buf = memory.data_mut(&mut caller);
                        let data_ptr = ptr as usize;
                        if data_ptr + 32 > data_buf.len() { return -3; }
                        data_buf[data_ptr..data_ptr+32].copy_from_slice(&msg.data);
                        if let Some(c) = msg.cap {
                            return add_cap_to_table(&mut caller, c);
                        }
                        return 0;
                    }
                    return -15;
                }
                return -14;
            }
            OP_LOOKUP => {
                // Step 7.1: Now requires a Registry handle
                if let Object::Registry = cap.obj {
                    if (cap.rights.0 & Rights::LOOKUP) == 0 { return -2; }
                    let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                    let data = memory.data(&caller);
                    let end = (ptr + len) as usize;
                    if end > data.len() { return -3; }
                    let name_slice = &data[ptr as usize..end];
                    if let Ok(name) = core::str::from_utf8(name_slice) {
                        if let Some(c) = crate::wasm::lookup_service(name) {
                            return add_cap_to_table(&mut caller, c);
                        }
                        return -16; // Service Not Found
                    }
                    return -4;
                }
                -17 // Not a Registry handle
            }
            OP_REGISTER => {
                // Step 7.1: Now requires a Registry handle
                if let Object::Registry = cap.obj {
                    if (cap.rights.0 & Rights::REGISTER) == 0 { return -2; }
                    
                    // ptr = target_handle, len = name_ptr (packed or extra call)
                    // For now, let's assume ptr is the target handle and we use an extra param
                    // but we are limited. 
                    // Let's use a struct in memory for REGISTER.
                    let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                    let data = memory.data(&caller);
                    let struct_ptr = ptr as usize;
                    if struct_ptr + 12 > data.len() { return -3; }
                    
                    let target_h = u32::from_le_bytes(data[struct_ptr..struct_ptr+4].try_into().unwrap());
                    let name_p = u32::from_le_bytes(data[struct_ptr+4..struct_ptr+8].try_into().unwrap()) as usize;
                    let name_l = u32::from_le_bytes(data[struct_ptr+8..struct_ptr+12].try_into().unwrap()) as usize;
                    
                    if name_p + name_l > data.len() { return -3; }
                    let name_slice = &data[name_p..name_p+name_l];
                    
                    let target_idx = (target_h & 0xFFFF) as usize;
                    let target_gen = (target_h >> 16) & 0xFFFF;
                    
                    let target_entry = match caller.data().handles.get(target_idx) {
                        Some(Some(e)) if e.generation == target_gen as u32 => e.clone(),
                        _ => return -1,
                    };

                    if let Ok(name) = core::str::from_utf8(name_slice) {
                        let pid = caller.data().pid;
                        crate::wasm::register_service(pid, name, target_entry.cap);
                        return 0;
                    }
                    return -4;
                }
                -17
            }
            OP_SPAWN => {
                // Step 8.3: Spawning requires Registry (Orchestration) rights
                if let Object::Registry = cap.obj {
                    if (cap.rights.0 & Rights::REGISTER) == 0 { return -2; }
                    
                    let memory = caller.get_export("memory").and_then(Extern::into_memory).expect("Wasm memory missing");
                    let data = memory.data(&caller);
                    let struct_ptr = ptr as usize;
                    if struct_ptr + 16 > data.len() { return -3; }
                    
                    let mod_p = u32::from_le_bytes(data[struct_ptr..struct_ptr+4].try_into().unwrap()) as usize;
                    let mod_l = u32::from_le_bytes(data[struct_ptr+4..struct_ptr+8].try_into().unwrap()) as usize;
                    let pid_val = u32::from_le_bytes(data[struct_ptr+8..struct_ptr+12].try_into().unwrap()) as usize;
                    
                    if mod_p + mod_l > data.len() { return -3; }
                    
                    // In a real system, we'd use a real loader. 
                    // For now, we'll use the kernel's helper with a mock manifest.
                    let wasm_bytes = &data[mod_p..mod_p+mod_l];
                    
                    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
                    let _ = write!(serial, "[Wasm] procd: Spawning PID {}\n", pid_val);
                    
                    // Spawn logic (using a copy of current engine/linker for simplicity)
                    // In real OS, this would be queued to the scheduler.
                    crate::wasm::spawn_from_orchestrator(pid_val, wasm_bytes);
                    return 0;
                }
                -17
            }
            OP_CLOSE => {
                caller.data_mut().handles[index] = None;
                return 0;
            }
            _ => return -10,
        }
    })?;

    Ok(())
}

fn add_cap_to_table(caller: &mut Caller<'_, WasmProcessState>, cap: crate::wasm::Capability) -> i32 {
    let handles = &mut caller.data_mut().handles;
    if let Some(i) = handles.iter().position(|h| h.is_none()) {
        handles[i] = Some(HandleEntry { cap, generation: 0 });
        return (i as i32) & 0xFFFF;
    } else {
        let new_idx = handles.len();
        handles.push(Some(HandleEntry { cap, generation: 0 }));
        return (new_idx as i32) & 0xFFFF;
    }
}
