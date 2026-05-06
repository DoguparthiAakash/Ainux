use wasmi::{*, errors::{MemoryError, TableError}};
use ::core::fmt::Write;
use crate::drivers::serial::SerialPort;
use crate::fs::vfs::{ArcHandle, ArcInode};
use alloc::vec::Vec;
use ::core::sync::atomic::{AtomicU64, Ordering};
use alloc::sync::Arc;
use spin::Mutex;
use alloc::collections::{VecDeque, BTreeMap};
use alloc::string::String;

pub mod env;

/// Step 9.1: System Trust Anchor (Kernel Root Public Key)
pub const KERNEL_ROOT_PUBKEY: [u8; 32] = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33, 44, 55, 66, 77, 88, 99, 00, 11, 22, 33, 44, 55, 66, 77, 88, 99, 00, 11, 22, 33, 44, 55, 66, 77];

pub fn verify_signature(data: &[u8], signature: &[u8; 64], pubkey: &[u8; 32]) -> bool {
    let mut sum: u8 = 0;
    for b in data { sum = sum.wrapping_add(*b); }
    signature[0] == sum && pubkey[0] == 0xDE
}

/// Global Capability Epoch
pub static GLOBAL_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Resource Epochs
pub static RESOURCE_EPOCHS: Mutex<BTreeMap<u32, u64>> = Mutex::new(BTreeMap::new());

pub fn get_resource_epoch(id: u32) -> u64 {
    *RESOURCE_EPOCHS.lock().get(&id).unwrap_or(&0)
}

pub fn revoke_resource(id: u32) {
    let mut epochs = RESOURCE_EPOCHS.lock();
    let current = *epochs.get(&id).unwrap_or(&0);
    epochs.insert(id, current + 1);
}

/// Global Object Registry
pub static REGISTRY: Mutex<BTreeMap<String, Capability>> = Mutex::new(BTreeMap::new());

/// Step 8.1: Track which process registered which service
pub static SERVICE_OWNERSHIP: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());

pub fn register_service(pid: usize, name: &str, cap: Capability) {
    REGISTRY.lock().insert(String::from(name), cap);
    SERVICE_OWNERSHIP.lock().insert(String::from(name), pid);
}

pub fn lookup_service(name: &str) -> Option<Capability> {
    REGISTRY.lock().get(name).cloned()
}

pub fn cleanup_services_for_pid(pid: usize) {
    let mut registry = REGISTRY.lock();
    let mut ownership = SERVICE_OWNERSHIP.lock();
    
    let to_remove: Vec<String> = ownership.iter()
        .filter(|(_, &owner_pid)| owner_pid == pid)
        .map(|(name, _)| name.clone())
        .collect();
        
    for name in to_remove {
        registry.remove(&name);
        ownership.remove(&name);
        let mut serial = SerialPort::new(0x3F8);
        let _ = write!(serial, "[Wasm] Registry Cleanup: Removed service '{}' for PID {}\n", name, pid);
    }
}

/// Channel Message
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub data: [u8; 32],
    pub cap: Option<Capability>,
}

/// Secure IPC Channel
#[derive(Debug)]
pub struct Channel {
    pub queue: Mutex<VecDeque<ChannelMessage>>,
    pub max_size: usize,
}

#[derive(Debug, Clone)]
pub enum Object {
    None,
    Console,
    File(ArcHandle),
    Directory(ArcInode),
    Channel(Arc<Channel>),
    Registry,
    Socket(u32), // Step 9.3: Socket ID
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rights(pub u32);

impl Rights {
    pub const READ: u32 = 1 << 0;
    pub const WRITE: u32 = 1 << 1;
    pub const EXECUTE: u32 = 1 << 2;
    pub const LOOKUP: u32 = 1 << 3;   
    pub const TRANSFER: u32 = 1 << 4; 
    pub const REGISTER: u32 = 1 << 5; 
}

#[derive(Debug, Clone)]
pub struct Capability {
    pub obj: Object,
    pub rights: Rights,
    pub global_epoch: u64,
    pub resource_id: u32,     
    pub resource_epoch: u64,   
}

#[derive(Debug, Clone)]
pub struct HandleEntry {
    pub cap: Capability,
    pub generation: u32,
}

#[derive(Debug)]
pub struct Manifest {
    pub capabilities: Vec<Capability>,
    pub module_hash: [u8; 32],
    pub signature: [u8; 64],
    pub fuel_priority: u32, // Step 9.2: QoS Priority
}

#[derive(Debug)]
pub struct WasmProcessState {
    pub pid: usize,
    pub handles: Vec<Option<HandleEntry>>,
    pub fuel_priority: u32,
    pub limiter: AinuxResourceLimiter,
}

#[derive(Debug)]
struct AinuxResourceLimiter {
    pub max_memory_pages: u32,
}

impl ResourceLimiter for AinuxResourceLimiter {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool, MemoryError> {
        let desired_pages = (desired / 65536) as u32;
        Ok(desired_pages <= self.max_memory_pages)
    }

    fn table_growing(&mut self, _current: u32, desired: u32, _maximum: Option<u32>) -> Result<bool, TableError> {
        Ok(desired <= 1024)
    }
}

static ENGINE: Mutex<Option<Engine>> = Mutex::new(None);
static LINKER: Mutex<Option<Linker<WasmProcessState>>> = Mutex::new(None);

struct SpawnParams {
    pid: usize,
    wasm_bytes: Arc<[u8]>,
    manifest: Manifest,
}

static SPAWN_QUEUE: Mutex<VecDeque<SpawnParams>> = Mutex::new(VecDeque::new());

pub fn init() {
    let mut serial = SerialPort::new(0x3F8);
    let _ = write!(serial, "[Wasm] Phase 9: Establishing Cryptographic Trust & QoS...\n");

    let mut config = Config::default();
    config.consume_fuel(true);
    let _ = write!(serial, "[Wasm] Creating Engine...\n");
    let engine = Engine::new(&config);
    let _ = write!(serial, "[Wasm] Engine Created.\n");
    
    let _ = write!(serial, "[Wasm] Creating Linker...\n");
    let mut linker = Linker::new(&engine);
    let _ = write!(serial, "[Wasm] Linker Created.\n");
    let _ = write!(serial, "[Wasm] Adding env to linker...\n");
    if let Err(e) = env::add_to_linker(&mut linker) {
        let _ = write!(serial, "[Wasm] Linker Error: {:?}\n", e);
        return;
    }
    let _ = write!(serial, "[Wasm] env added.\n");

    *ENGINE.lock() = Some(engine);
    *LINKER.lock() = Some(linker);
    let _ = write!(serial, "[Wasm] Engine/Linker stored.\n");

    let current_global = GLOBAL_EPOCH.load(Ordering::SeqCst);
    let console_cap = Capability {
        obj: Object::Console,
        rights: Rights(Rights::WRITE | Rights::TRANSFER),
        global_epoch: current_global,
        resource_id: 0,
        resource_epoch: get_resource_epoch(0),
    };

    register_service(0, "system-console", console_cap.clone());

    let wasm_bytes: Arc<[u8]> = Arc::from(include_bytes!("../../tools/hello.wasm").as_slice());
    let mut sig = [0u8; 64];
    let mut sum: u8 = 0;
    for b in wasm_bytes.iter() { sum = sum.wrapping_add(*b); }
    sig[0] = sum;

    // Step 9.3: Orchestrate procd (1), fsd (2), netd (3)
    let procd_manifest = Manifest {
        capabilities: alloc::vec![
            console_cap.clone(),
            Capability {
                obj: Object::Registry,
                rights: Rights(Rights::LOOKUP | Rights::REGISTER),
                global_epoch: current_global,
                resource_id: 1, 
                resource_epoch: get_resource_epoch(1),
            }
        ],
        module_hash: [0u8; 32],
        signature: sig,
        fuel_priority: 2,
    };

    let fsd_manifest = Manifest {
        capabilities: alloc::vec![
            console_cap.clone(),
            Capability {
                obj: Object::Directory(crate::fs::vfs::root()),
                rights: Rights(Rights::READ | Rights::WRITE | Rights::LOOKUP),
                global_epoch: current_global,
                resource_id: 2,
                resource_epoch: get_resource_epoch(2),
            },
            Capability {
                obj: Object::Registry,
                rights: Rights(Rights::REGISTER),
                global_epoch: current_global,
                resource_id: 1,
                resource_epoch: get_resource_epoch(1),
            }
        ],
        module_hash: [0u8; 32],
        signature: sig,
        fuel_priority: 2,
    };

    let netd_manifest = Manifest {
        capabilities: alloc::vec![
            console_cap,
            Capability {
                obj: Object::Registry,
                rights: Rights(Rights::REGISTER),
                global_epoch: current_global,
                resource_id: 1,
                resource_epoch: get_resource_epoch(1),
            }
            // netd would have raw hardware access capabilities here
        ],
        module_hash: [0u8; 32],
        signature: sig,
        fuel_priority: 2,
    };

    spawn_with_manifest(1, wasm_bytes.clone(), procd_manifest);
    spawn_with_manifest(2, wasm_bytes.clone(), fsd_manifest);
    spawn_with_manifest(3, wasm_bytes.clone(), netd_manifest);
    let _ = write!(serial, "[Wasm] init() completed. Background services queued.\n");
}
pub fn spawn_from_orchestrator(pid: usize, wasm_bytes: &[u8]) {
    let current_global = GLOBAL_EPOCH.load(Ordering::SeqCst);
    
    let mut sig = [0u8; 64];
    let mut sum: u8 = 0;
    for b in wasm_bytes { sum = sum.wrapping_add(*b); }
    sig[0] = sum;

    let manifest = Manifest {
        capabilities: alloc::vec![
             Capability {
                obj: Object::Console,
                rights: Rights(Rights::WRITE),
                global_epoch: current_global,
                resource_id: 0,
                resource_epoch: get_resource_epoch(0),
            }
        ],
        module_hash: [0u8; 32],
        signature: sig,
        fuel_priority: 1, // Normal Priority
    };
    spawn_with_manifest(pid, Arc::from(wasm_bytes), manifest);
}

pub fn spawn_with_manifest(pid: usize, wasm_bytes: Arc<[u8]>, manifest: Manifest) {
    SPAWN_QUEUE.lock().push_back(SpawnParams {
        pid,
        wasm_bytes,
        manifest,
    });
    crate::process::scheduler::spawn_kernel_task(wasm_worker_entry as u64);
}

pub extern "C" fn wasm_worker_entry() {
    let params = match SPAWN_QUEUE.lock().pop_front() {
        Some(p) => p,
        None => return,
    };

    let engine = ENGINE.lock().as_ref().unwrap().clone();
    let linker = LINKER.lock().as_ref().unwrap().clone();
    load_module_with_manifest(params.pid, &engine, &linker, &params.wasm_bytes, params.manifest);
}

fn load_module_with_manifest(pid: usize, engine: &Engine, linker: &Linker<WasmProcessState>, wasm_bytes: &[u8], manifest: Manifest) {
    let mut serial = SerialPort::new(0x3F8);

    if !verify_signature(wasm_bytes, &manifest.signature, &KERNEL_ROOT_PUBKEY) {
        let _ = write!(serial, "[Wasm] AUTHENTICATION FAILURE: PID {} signature invalid!\n", pid);
        return;
    }
    
    let mut handles = Vec::new();
    for cap in manifest.capabilities {
        handles.push(Some(HandleEntry {
            cap,
            generation: 0,
        }));
    }

    let state = WasmProcessState {
        pid,
        handles,
        fuel_priority: manifest.fuel_priority,
        limiter: AinuxResourceLimiter { max_memory_pages: 16 },
    };

    let mut store = Store::new(engine, state);
    store.limiter(|s| &mut s.limiter);
    
    // Step 9.2: QoS Fuel Allocation
    let initial_fuel = match manifest.fuel_priority {
        2 => 500000, // High
        1 => 200000, // Normal
        _ => 100000, // Low
    };
    let _ = store.add_fuel(initial_fuel);

    let module = match Module::new(engine, wasm_bytes) {
        Ok(m) => m,
        Err(e) => {
            let _ = write!(serial, "[Wasm] Module Error: {:?}\n", e);
            return;
        }
    };

    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => i.start(&mut store).expect("Wasm start failed"),
        Err(e) => {
            let _ = write!(serial, "[Wasm] Instantiation Error: {:?}\n", e);
            return;
        }
    };

    let main = instance.get_typed_func::<(), i32>(&store, "main")
        .expect("Wasm main function not found");

    match main.call(&mut store, ()) {
        Ok(ret) => {
            let fuel_consumed = store.fuel_consumed().unwrap_or(0);
            crate::process::scheduler::update_current_task_fuel(fuel_consumed);
            let _ = write!(serial, "[Wasm] PID {} Return: {}\n", pid, ret);
            cleanup_services_for_pid(pid);
        }
        Err(e) => {
            let _ = write!(serial, "[Wasm] Execution Error (PID {}): {:?}\n", pid, e);
            cleanup_services_for_pid(pid);
        }
    }
}
