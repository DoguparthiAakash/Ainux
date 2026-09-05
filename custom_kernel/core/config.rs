extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use spin::Mutex;
use alloc::sync::Arc;
use crate::fs::vfs::{self, ArcInode};
use core::fmt::Write;

pub struct SystemConfig {
    pub hostname: String,
    pub ip_address: String,
    pub gateway: String,
    pub dns: String,
    pub theme_color: u32,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            hostname: String::from("ainux-node"),
            ip_address: String::from("10.0.2.15"),
            gateway: String::from("10.0.2.2"),
            dns: String::from("8.8.8.8"),
            theme_color: 0x0000AAFF,
        }
    }
}

pub static CONFIG: Mutex<Option<SystemConfig>> = Mutex::new(None);

pub fn load() {
    // Ensure initialized
    {
        let mut config = CONFIG.lock();
        if config.is_none() {
            *config = Some(SystemConfig::default());
        }
    }

    let root = vfs::ROOT.lock();
    if let Some(r) = root.as_ref() {
        // Ensure /etc exists
        if let Err(_) = r.lookup("etc") {
            let _ = r.mkdir("etc");
        }

        match r.lookup("etc/ainux.conf") {
            Ok(inode) => {
                if let Ok(handle) = inode.open(0) {
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = handle.read(&mut buf, 0) {
                        if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                            parse_config(s);
                        }
                    }
                }
            },
            Err(_) => {
                // No config file, use defaults
            }
        }
    }
}

pub fn save() {
    let root = vfs::ROOT.lock();
    if let Some(r) = root.as_ref() {
        let etc = match r.lookup("etc") {
            Ok(e) => e,
            Err(_) => r.mkdir("etc").expect("Failed to create /etc"),
        };

        let inode = match etc.lookup("ainux.conf") {
            Ok(i) => i,
            Err(_) => etc.create("ainux.conf", vfs::FileType::File).expect("Failed to create config file"),
        };

        if let Ok(handle) = inode.open(0) {
            let _ = handle.truncate();
            if let Some(config) = CONFIG.lock().as_ref() {
                let data = format!(
                    "hostname={}\nip={}\ngateway={}\ndns={}\ntheme={}\n",
                    config.hostname, config.ip_address, config.gateway, config.dns, config.theme_color
                );
                let _ = handle.write(data.as_bytes(), 0);
            }
        }
    }
}

fn parse_config(s: &str) {
    if let Some(config) = CONFIG.lock().as_mut() {
        for line in s.lines() {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                match parts[0] {
                    "hostname" => config.hostname = String::from(parts[1]),
                    "ip" => config.ip_address = String::from(parts[1]),
                    "gateway" => config.gateway = String::from(parts[1]),
                    "dns" => config.dns = String::from(parts[1]),
                    "theme" => {
                        if let Ok(val) = u32::from_str_radix(parts[1], 10) {
                            config.theme_color = val;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn reset_to_defaults() {
    let mut config = CONFIG.lock();
    *config = Some(SystemConfig::default());
}
