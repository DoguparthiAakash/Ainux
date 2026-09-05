use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use spin::Mutex;

lazy_static::lazy_static! {
    pub static ref DATA_MANAGER: Arc<Mutex<DataManager>> = Arc::new(Mutex::new(DataManager::new()));
}

pub struct DataManager {
    data: BTreeMap<String, String>,
    backend_file: String,
}

impl DataManager {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            backend_file: String::from("/system/config.dat"),
        }
    }

    pub fn init(&mut self) {
        // Todo: Load from backend_file via VFS
        // For now, prepopulate with defaults
        self.data.insert("theme".to_string(), "dark".to_string());
        self.data.insert("hostname".to_string(), "ainux-pc".to_string());
    }

    pub fn get(&self, key: &str) -> String {
        self.data.get(key).cloned().unwrap_or(String::new())
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
        // Todo: Save to backend_file via VFS
    }
}
