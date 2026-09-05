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

#[derive(Clone, Debug)]
pub struct Group {
    pub gid: u32,
    pub name: String,
    pub members: Vec<u32>, // UIDs of members
}

pub struct UserManager {
    users: Vec<User>,
    groups: Vec<Group>,
    current_uid: u32, // Per-Process/Thread? For now, global session
}

impl UserManager {
    pub fn new() -> Self {
        let mut um = Self {
            users: Vec::new(),
            groups: Vec::new(),
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

    pub fn delete_user(&mut self, username: &str) -> bool {
        if let Some(pos) = self.users.iter().position(|u| u.username == username) {
            // Cannot delete root
            if self.users[pos].uid == 0 {
                return false;
            }
            let uid = self.users[pos].uid;
            self.users.remove(pos);
            
            // Remove from groups
            for g in &mut self.groups {
                g.members.retain(|&m| m != uid);
            }
            true
        } else {
            false
        }
    }

    pub fn create_group(&mut self, group_name: &str) -> bool {
        for g in &self.groups {
            if g.name == group_name { return false; }
        }
        
        let new_gid = self.groups.len() as u32;
        self.groups.push(Group {
            gid: new_gid,
            name: group_name.to_string(),
            members: Vec::new(),
        });
        true
    }

    pub fn add_user_to_group(&mut self, username: &str, group_name: &str) -> bool {
        let uid = match self.get_uid_by_name(username) {
            Some(id) => id,
            None => return false,
        };
        
        for g in &mut self.groups {
            if g.name == group_name {
                if !g.members.contains(&uid) {
                    g.members.push(uid);
                }
                return true;
            }
        }
        false
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

    pub fn get_uid_by_name(&self, username: &str) -> Option<u32> {
        for u in &self.users {
            if u.username == username {
                return Some(u.uid);
            }
        }
        None
    }

    pub fn get_user_by_name(&self, username: &str) -> Option<User> {
        for u in &self.users {
            if u.username == username {
                return Some(u.clone());
            }
        }
        None
    }
}
