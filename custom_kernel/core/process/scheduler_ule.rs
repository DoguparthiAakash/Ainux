// FreeBSD ULE Scheduler Port for Ainux (Rust)
// Based on sys/kern/sched_ule.c
// 
// ULE: Everything is about Thread Queues (TDQ) and Run Queues (Runq).
// It prioritizes interactivity and fairness using two queues (Current, Next).

use alloc::vec::Vec;
use spin::Mutex;

// Priorities (FreeBSD Defaults)
const PRI_MAX_INTERACT: u8 = 88;
const PRI_MIN_INTERACT: u8 = 48;
const PRI_MIN_BATCH: u8 = 104;
const PRI_MAX_BATCH: u8 = 224;

#[derive(Clone, Copy, PartialEq)]
pub enum TdState {
    InActive,
    Running,
    Runnable,
    Sleeping,
}

// Struct Thread (td)
pub struct Thread {
    pub tid: usize,
    pub priority: u8,
    pub state: TdState,
    pub slice: u32,      // Time slice remaining
    pub cpu_id: usize,   // CPU affinity
    pub interactivity: i32, // Score for interactivity
}

// Run Queue (runq)
pub struct Runq {
    queues: [Vec<Thread>; 256], // Bitmap priority queue
}

impl Runq {
    pub fn new() -> Self {
         // Create array of empty Vecs
         let queues: [Vec<Thread>; 256] = core::array::from_fn(|_| Vec::new());
         Self {
             queues,
         }
    }
    
    pub fn add(&mut self, td: Thread) {
        if (td.priority as usize) < 256 {
            self.queues[td.priority as usize].push(td);
        }
    }
    
    pub fn choose(&mut self) -> Option<Thread> {
        // Find highest priority non-empty queue (lowest index = higher priority)
        for i in 0..256 {
            if !self.queues[i].is_empty() {
                return self.queues[i].pop(); // FIFO for same priority
            }
        }
        None
    }
}

// Thread Queue (tdq) - Per CPU
pub struct Tdq {
    pub lock: Mutex<()>,
    pub runq: Runq,      // Timeshare Queue
    pub real_runq: Runq, // Realtime Queue
    pub load: usize,     // Load balancing metric
}

impl Tdq {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            runq: Runq::new(),
            real_runq: Runq::new(),
            load: 0,
        }
    }
}

// Global ULE Scheduler Instance (Lazy)
// We need Option because Vec allocation is not possible in const context currently
// without complex workarounds.
pub static ULE_GLOBAL: Mutex<Option<UleScheduler>> = Mutex::new(None);

pub struct UleScheduler {
    pub tdqs: [Mutex<Tdq>; 1], 
}

impl UleScheduler {
    pub fn init() {
        let mut guard = ULE_GLOBAL.lock();
        *guard = Some(UleScheduler {
            tdqs: [Mutex::new(Tdq::new())],
        });
    }
    
    pub fn add_thread(td: Thread) {
        let guard = ULE_GLOBAL.lock();
        if let Some(sched) = guard.as_ref() {
            let mut tdq = sched.tdqs[0].lock();
            if td.priority < PRI_MIN_BATCH {
                tdq.real_runq.add(td);
            } else {
                tdq.runq.add(td);
            }
            tdq.load += 1;
        }
    }
    
    pub fn pick_next() -> Option<Thread> {
        let guard = ULE_GLOBAL.lock();
        if let Some(sched) = guard.as_ref() {
            let mut tdq = sched.tdqs[0].lock();
            // Try Realtime first
            if let Some(t) = tdq.real_runq.choose() {
                tdq.load -= 1;
                return Some(t);
            }
            // Try Timeshare
            if let Some(t) = tdq.runq.choose() {
                tdq.load -= 1;
                return Some(t);
            }
        }
        None
    }
}
