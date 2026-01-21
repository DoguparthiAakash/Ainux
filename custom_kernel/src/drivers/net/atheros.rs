use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use spin::Mutex;
use crate::drivers::iokit::service::IOService;
use crate::drivers::iokit::types::{IOValue, IOResult};
use crate::drivers::video;

#[derive(Debug, Clone)]
struct Network {
    ssid: String,
    signal: u8,
    encrypted: bool,
}

#[derive(Debug)]
pub struct AtherosHAL {
    name: String,
    connected_ssid: Mutex<Option<String>>,
    networks: Mutex<Vec<Network>>,
}

pub static GLOBAL_ATHEROS: Mutex<Option<Arc<AtherosHAL>>> = Mutex::new(None);

impl AtherosHAL {
    pub fn new() -> Arc<Self> {
        let mut nets = Vec::new();
        nets.push(Network { ssid: String::from("Ainux-5G"), signal: 90, encrypted: true });
        nets.push(Network { ssid: String::from("Airtel_Hema"), signal: 95, encrypted: true });
        nets.push(Network { ssid: String::from("Guest-WiFi"), signal: 60, encrypted: false });
        nets.push(Network { ssid: String::from("Neighbor-Net"), signal: 20, encrypted: true });
        
        let driver = Arc::new(Self { 
            name: String::from("Atheros AR9271 Wireless"),
            connected_ssid: Mutex::new(None),
            networks: Mutex::new(nets),
        });
        
        *GLOBAL_ATHEROS.lock() = Some(driver.clone());
        driver
    }
    
    pub fn scan(&self) -> String {
        String::from("Scanning... [Found: Ainux-5G, Airtel_Hema, Guest-WiFi]\n")
    }
    
    pub fn connect(&self, ssid: &str, _password: &str) -> String {
        let nets = self.networks.lock();
        let mut found = false;
        for net in nets.iter() {
            if net.ssid == ssid {
                found = true;
                break;
            }
        }
        
        if found {
            *self.connected_ssid.lock() = Some(String::from(ssid));
            format!("Authenticated. Associated with '{}'. IP obtained.\n", ssid)
        } else {
            String::from("Error: Network not found.\n")
        }
    }
    
    pub fn get_status(&self) -> String {
        let conn = self.connected_ssid.lock();
        match &*conn {
            Some(ssid) => format!("State: CONNECTED to '{}' (RSSI: -45dBm)\n", ssid),
            None => String::from("State: DISCONNECTED (Radio On)\n"),
        }
    }
}

impl IOService for AtherosHAL {
    fn get_name(&self) -> &str { &self.name }
    
    fn get_property(&self, _key: &str) -> Option<IOValue> { None }
    
    fn probe(&self, _provider: &Arc<dyn IOService>) -> i32 {
        // Always attach if manually requested, or we could match a simulated VID/DID
        100 
    }
    
    fn start(&self, _provider: &Arc<dyn IOService>) -> IOResult<()> {
        video::put_str("Atheros: Radio Initialized. Firmware Loaded (v2.1).\n");
        Ok(())
    }
    
    fn stop(&self) {
        video::put_str("Atheros: Radio Off.\n");
    }
}
