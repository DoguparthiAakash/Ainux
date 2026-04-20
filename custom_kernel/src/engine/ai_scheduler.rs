use alloc::vec::Vec;
use spin::Mutex;
use crate::process::scheduler::TASKS;

// "AI" State for a process
#[derive(Debug, Clone, Copy)]
pub struct ProcessMood {
    pub anger: f32, // CPU Usage (0.0 - 1.0)
    pub hunger: f32, // Memory Usage (0.0 - 1.0)
    pub predicted_anger: f32, // Next tick prediction
}

pub struct ResourceManager;

impl ResourceManager {
    pub fn update_heuristics() {
        let mut lock = TASKS.lock();
        if let Some(tasks) = lock.as_mut() {
            for task in tasks.iter_mut() {
                if task.state != crate::process::task::TaskState::Free {
                    let _current_load = if task.state == crate::process::task::TaskState::Running { 0.1 } else { 0.0 };
                }
            }
        }
    }
    
    // The "OOM Killer" but smarter?
    pub fn maximize_efficiency() {
        // Remove Zombies
        // Reclaim Frames
    }
}
