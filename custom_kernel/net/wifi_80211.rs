// src/net/wifi_80211.rs
// IEEE 802.11 Management Frame Parser for Ainux

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct BeaconInfo {
    pub ssid: String,
    pub bssid: [u8; 6],
    pub signal: i8,
}

pub fn parse_beacon(data: &[u8]) -> Option<BeaconInfo> {
    // 802.11 Management Frame (Subtype 8 = Beacon)
    if data.len() < 24 { return None; }
    
    let frame_control = u16::from_le_bytes([data[0], data[1]]);
    let subtype = (frame_control >> 4) & 0x0F;
    if subtype != 8 { return None; } // Not a beacon

    let bssid = [data[16], data[17], data[18], data[19], data[20], data[21]];
    
    // Tagged Parameters start at offset 36
    let mut pos = 36;
    let mut ssid = String::from("<Hidden>");
    
    while pos + 2 <= data.len() {
        let tag_id = data[pos];
        let tag_len = data[pos+1] as usize;
        pos += 2;
        
        if pos + tag_len > data.len() { break; }
        
        if tag_id == 0 { // SSID Tag
            let ssid_bytes = &data[pos..pos+tag_len];
            if let Ok(s) = core::str::from_utf8(ssid_bytes) {
                ssid = String::from(s);
            }
            break; // Found SSID, we can stop for now
        }
        
        pos += tag_len;
    }

    Some(BeaconInfo {
        ssid,
        bssid,
        signal: -65, // Placeholder for real RSSI from hardware
    })
}
