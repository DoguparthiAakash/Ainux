use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use spin::Mutex;

lazy_static::lazy_static! {
    pub static ref USER_MANAGER: Arc<Mutex<UserManager>> = Arc::new(Mutex::new(UserManager::new()));
}

#[derive(Clone, Debug)]
pub struct User {
    pub uid: u32,
    pub username: String,
    password_hash: String, // Simple plain text for now
    pub permissions: u32, // Bitmask
}

pub struct UserManager {
    users: Vec<User>,
    current_uid: u32, // Per-Process/Thread? For now, global session
}

impl UserManager {
    pub fn new() -> Self {
        let mut um = Self {
            users: Vec::new(),
            current_uid: 0, // Default to root/admin
        };
        // Default Sovereign User
        um.users.push(User {
            uid: 0,
            username: "root".to_string(),
            password_hash: "ainux".to_string(),
            permissions: 0xFFFFFFFF,
        });
        um
    }

    pub fn authenticate(&mut self, user: &str, pass: &str) -> bool {
        for u in &self.users {
            if u.username == user && u.password_hash == pass {
                self.current_uid = u.uid;
                return true;
            }
        }
        false
    }
    
    pub fn get_current_user(&self) -> String {
         for u in &self.users {
             if u.uid == self.current_uid {
                 return u.username.clone();
             }
         }
         "unknown".to_string()
    }

    pub fn create_user(&mut self, user: &str, pass: &str) -> bool {
        // Check if exists
        for u in &self.users {
            if u.username == user { return false; }
        }
        
        let new_uid = self.users.len() as u32;
        self.users.push(User {
            uid: new_uid,
            username: user.to_string(),
            password_hash: pass.to_string(),
            permissions: 1, // Basic
        });
        true
    }

    pub fn update_password(&mut self, user: &str, new_pass: &str) -> bool {
        for u in &mut self.users {
            if u.username == user {
                u.password_hash = new_pass.to_string();
                return true;
            }
        }
        false
    }
}
