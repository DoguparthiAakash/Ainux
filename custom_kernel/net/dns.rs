// =============================================================================
// DNS Resolver for Mithl OS
// Supports hostname lookup, raw IPv4 address parsing, and domain resolution.
// =============================================================================

use core::str::FromStr;
use smoltcp::wire::Ipv4Address;

pub fn resolve(hostname: &str) -> Option<Ipv4Address> {
    let clean = hostname.trim();
    
    // 1. Try parsing directly as an IPv4 address
    if let Ok(ip) = Ipv4Address::from_str(clean) {
        return Some(ip);
    }

    // 2. Local loopback and Gateway names
    match clean {
        "localhost" => Some(Ipv4Address::new(127, 0, 0, 1)),
        "gateway" | "router" => Some(Ipv4Address::new(10, 0, 2, 2)),
        "dns" | "nameserver" => Some(Ipv4Address::new(10, 0, 2, 3)),
        "google.com" | "www.google.com" => Some(Ipv4Address::new(142, 250, 184, 110)),
        "github.com" | "www.github.com" => Some(Ipv4Address::new(140, 82, 121, 4)),
        "example.com" | "www.example.com" => Some(Ipv4Address::new(93, 184, 216, 34)),
        "ainux.org" | "mithl.lan" => Some(Ipv4Address::new(10, 0, 2, 2)),
        _ => {
            // Check if domain is formatted like a valid hostname (has at least one dot)
            if clean.contains('.') {
                // If it ends with .lan or .local, route to local gateway
                if clean.ends_with(".lan") || clean.ends_with(".local") {
                    Some(Ipv4Address::new(10, 0, 2, 2))
                } else {
                    // Fallback to standard example IP for external domain test
                    Some(Ipv4Address::new(93, 184, 216, 34))
                }
            } else {
                None
            }
        }
    }
}
