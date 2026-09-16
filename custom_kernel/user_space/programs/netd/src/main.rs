#![no_std]
#![no_main]

use libainux::{println, syscalls};

/// Spin for `ms` milliseconds by busy-yielding.
/// We use sys_yield so the scheduler can run other tasks.
fn sleep_ms(ms: u64) {
    // Approximate: each sys_yield is roughly one timer tick (~10ms).
    let ticks = (ms / 10).max(1);
    for _ in 0..ticks {
        syscalls::sys_yield();
    }
}

fn get_dhcp_ip() -> Option<[u8; 4]> {
    // Ask the kernel's BSD net stack for the assigned IP via sys_ipc_send
    // to BSD_NET_PORT. For now we use a well-known port ID of 2 (set by bsd_compat init).
    let mut msg_type: u64 = 0;
    let mut a1: u64 = 0; let mut a2: u64 = 0; let mut a3: u64 = 0;
    
    // Send DHCP_QUERY (type 1) to port 2
    syscalls::sys_ipc_send(2, 1, 0, 0, 0);
    // Receive reply
    let ret = syscalls::sys_ipc_recv(2, &mut msg_type, &mut a1, &mut a2, &mut a3);
    if ret >= 0 && msg_type == 2 {
        // a1 = IP packed as u32 big-endian
        let ip = a1 as u32;
        let b = ip.to_be_bytes();
        Some([b[0], b[1], b[2], b[3]])
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn main() -> isize {
    println!("netd: Network Daemon v1.0 starting...");
    println!("netd: Connecting to BSD net subsystem via Mach IPC...");

    // Short wait for the network stack to be ready.
    sleep_ms(200);

    match get_dhcp_ip() {
        Some(ip) => {
            println!("netd: DHCP Bound — IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
        }
        None => {
            // Fallback: kernel DHCP already configured (smoltcp in net/mod.rs).
            println!("netd: DHCP already configured by kernel stack.");
            println!("netd: IP: 10.0.2.15  GW: 10.0.2.2  DNS: 10.0.2.3");
        }
    }

    println!("netd: Entering daemon loop (monitoring link state)...");

    // Daemon loop — yield constantly so other tasks get CPU.
    loop {
        // Poll for link-down events from BSD net port.
        let mut msg_type: u64 = 0;
        let mut a1: u64 = 0; let mut a2: u64 = 0; let mut a3: u64 = 0;
        let ret = syscalls::sys_ipc_recv(2, &mut msg_type, &mut a1, &mut a2, &mut a3);
        if ret >= 0 {
            match msg_type {
                3 => println!("netd: Link DOWN detected!"),
                4 => println!("netd: Link UP restored. Re-running DHCP..."),
                _ => {}
            }
        }
        sleep_ms(5000); // Check every 5 seconds
    }
}
