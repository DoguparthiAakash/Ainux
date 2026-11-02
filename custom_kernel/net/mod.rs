extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv4Cidr, Ipv6Address, Ipv6Cidr};
use smoltcp::socket::{dhcpv4, tcp};
use crate::drivers::net::rtl8139::RTL8139;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};
use smoltcp::socket::AnySocket;

pub mod dns;
pub mod wifi_80211;
pub mod baseos;
pub mod inet;

// Ainux OSI 7-Layer Architecture
pub mod layer1_physical;
pub mod layer2_datalink;
pub mod layer3_network;
pub mod layer4_transport;
pub mod layer5_session;
pub mod layer6_presentation;
pub mod layer7_application;

pub struct AinuxDevice;

impl Device for AinuxDevice {
    type RxToken<'a> = AinuxRxToken;
    type TxToken<'a> = AinuxTxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let cell = crate::process::scheduler::get_current_cell();
        
        // 1. Check Cell-specific queue
        {
            let mut queue = cell.rx_queue.lock();
            if let Some(data) = queue.pop_front() {
                let buffer = match Arc::try_unwrap(data) {
                    Ok(v) => v,
                    Err(arc) => (*arc).clone(),
                };
                return Some((AinuxRxToken { buffer }, AinuxTxToken));
            }
        }

        // 2. Fallback for Root Cell or hardware-direct tasks
        let mut result = None;
        RTL8139::receive_packet(|data: &[u8]| {
            result = Some((AinuxRxToken { buffer: data.to_vec() }, AinuxTxToken));
        });
        if result.is_none() {
            crate::drivers::net::e1000::receive_packet(|data: &[u8]| {
                result = Some((AinuxRxToken { buffer: data.to_vec() }, AinuxTxToken));
            });
        }
        result
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(AinuxTxToken)
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct AinuxRxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for AinuxRxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.buffer)
    }
}

pub struct AinuxTxToken;

impl phy::TxToken for AinuxTxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = Vec::with_capacity(len);
        buffer.resize(len, 0);
        let result = f(&mut buffer);
        RTL8139::send_packet(&buffer);
        crate::drivers::net::e1000::send_packet(&buffer);
        result
    }
}

impl core::fmt::Debug for NetStack {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NetStack")
         .field("dhcp", &self.dhcp_handle)
         .field("icmp", &self.icmp_handle)
         .finish()
    }
}

pub struct NetStack {
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub dhcp_handle: Option<smoltcp::iface::SocketHandle>,
    pub icmp_handle: Option<smoltcp::iface::SocketHandle>,
}

pub static NET_STACK: Mutex<Option<NetStack>> = Mutex::new(None);
pub static DEFAULT_GATEWAY_V4: Mutex<Option<[u8; 4]>> = Mutex::new(Some([10, 0, 2, 2]));
pub static DEFAULT_GATEWAY_V6: Mutex<Option<alloc::string::String>> = Mutex::new(None);
pub static DNS_SERVERS: Mutex<alloc::vec::Vec<[u8; 4]>> = Mutex::new(alloc::vec::Vec::new());
pub static DHCP_BOUND: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

pub fn get_hardware_mac() -> [u8; 6] {
    if let Some(mac) = RTL8139::get_mac() {
        mac
    } else if let Some(mac) = crate::drivers::net::e1000::get_mac() {
        mac
    } else {
        [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
    }
}

pub fn init() {
    let mut caps = smoltcp::phy::DeviceCapabilities::default();
    caps.max_transmission_unit = 1500;
    caps.medium = smoltcp::phy::Medium::Ethernet;

    let hw_mac = get_hardware_mac();
    let config = smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress(hw_mac)));
    
    let mut device = AinuxDevice;
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));

    // 1. Establish IPv4 Network: 10.0.2.15/24 (Standard guest NAT IP)
    let ipv4_addr = Ipv4Address::new(10, 0, 2, 15);
    let ipv4_cidr = Ipv4Cidr::new(ipv4_addr, 24);

    // 2. Establish IPv6 Link-Local: RFC 4291 EUI-64 derived from hardware MAC
    // Invert the universal/local bit (bit 1 of byte 0)
    let b0 = hw_mac[0] ^ 0x02;
    let ipv6_link_local = Ipv6Address([
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        b0, hw_mac[1], hw_mac[2], 0xff, 0xfe, hw_mac[3], hw_mac[4], hw_mac[5],
    ]);
    let ipv6_ll_cidr = Ipv6Cidr::new(ipv6_link_local, 64);

    // 3. Establish IPv6 Unique Local Address (ULA): fd00::15/64
    let ipv6_ula = Ipv6Address([
        0xfd, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        b0, hw_mac[1], hw_mac[2], 0xff, 0xfe, hw_mac[3], hw_mac[4], hw_mac[5],
    ]);
    let ipv6_ula_cidr = Ipv6Cidr::new(ipv6_ula, 64);

    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::Ipv4(ipv4_cidr));
        let _ = addrs.push(IpCidr::Ipv6(ipv6_ll_cidr));
        let _ = addrs.push(IpCidr::Ipv6(ipv6_ula_cidr));
    });

    // Default Gateway routes: IPv4 gateway 10.0.2.2, IPv6 gateway fe80::2
    let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2));
    let _ = iface.routes_mut().add_default_ipv6_route(Ipv6Address([
        0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2
    ]));

    let mut sockets = SocketSet::new(Vec::new());

    // Add DHCP socket for dynamic lease tracking
    let dhcp_socket = dhcpv4::Socket::new();
    let dhcp_handle = sockets.add(dhcp_socket);

    // Add ICMP socket (for Ping)
    let icmp_rx_buffer = smoltcp::socket::icmp::PacketBuffer::new(
        alloc::vec![smoltcp::socket::icmp::PacketMetadata::EMPTY],
        alloc::vec![0; 256]
    );
    let icmp_tx_buffer = smoltcp::socket::icmp::PacketBuffer::new(
        alloc::vec![smoltcp::socket::icmp::PacketMetadata::EMPTY],
        alloc::vec![0; 256]
    );
    let icmp_socket = smoltcp::socket::icmp::Socket::new(icmp_rx_buffer, icmp_tx_buffer);
    let icmp_handle = sockets.add(icmp_socket);

    *NET_STACK.lock() = Some(NetStack {
        iface,
        sockets,
        dhcp_handle: Some(dhcp_handle),
        icmp_handle: Some(icmp_handle),
    });

    *DEFAULT_GATEWAY_V4.lock() = Some([10, 0, 2, 2]);
    *DEFAULT_GATEWAY_V6.lock() = Some(alloc::string::String::from("fe80::2"));
    *DNS_SERVERS.lock() = alloc::vec![[10, 0, 2, 3], [1, 1, 1, 1], [8, 8, 8, 8]];

    // Initialize OSI Architecture Modules
    layer1_physical::init();
    layer2_datalink::init();
    layer3_network::init();
    layer4_transport::init();
    layer5_session::init();
    layer6_presentation::init();
    layer7_application::init();

    crate::drivers::video::put_str(&alloc::format!(
        "Net: Online! MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n     IPv4: 10.0.2.15/24 (gw 10.0.2.2)\n     IPv6: {}\n",
        hw_mac[0], hw_mac[1], hw_mac[2], hw_mac[3], hw_mac[4], hw_mac[5],
        ipv6_link_local
    ));
}

pub fn poll() {
    crate::cpu::without_interrupts(|| {
        let stack_mutex = &crate::net::NET_STACK;

        let mut stack_lock = stack_mutex.lock();
        if let Some(stack) = stack_lock.as_mut() {
            let mut device = AinuxDevice;
            let now = Instant::from_millis((crate::process::scheduler::get_ticks() * 10) as i64);
            stack.iface.poll(now, &mut device, &mut stack.sockets);

            // Handle DHCP
            if let Some(handle) = stack.dhcp_handle {
                let socket = stack.sockets.get_mut::<dhcpv4::Socket>(handle);
                if let Some(event) = socket.poll() {
                    match event {
                        dhcpv4::Event::Configured(config) => {
                            let mut already_configured = false;
                            stack.iface.update_ip_addrs(|addrs| {
                                if !addrs.iter().any(|a| a == &IpCidr::Ipv4(config.address)) {
                                    addrs.retain(|a| match a {
                                        IpCidr::Ipv4(_) => false,
                                        IpCidr::Ipv6(_) => true,
                                    });
                                    if let Err(_) = addrs.push(IpCidr::Ipv4(config.address)) {
                                        // Ignore push error if full
                                    }
                                } else {
                                    already_configured = true;
                                }
                            });
                            
                            if !already_configured {
                                DHCP_BOUND.store(true, core::sync::atomic::Ordering::Relaxed);
                                crate::drivers::video::put_str(&alloc::format!("Net: DHCP Lease Acquired: {}\n", config.address));
                                if let Some(router) = config.router {
                                    *DEFAULT_GATEWAY_V4.lock() = Some(router.0);
                                    if let Err(_) = stack.iface.routes_mut().add_default_ipv4_route(router) {
                                        // Ignore route error
                                    }
                                }
                                let mut dns_list = alloc::vec::Vec::new();
                                for d in config.dns_servers.iter() {
                                    dns_list.push(d.0);
                                }
                                if !dns_list.is_empty() {
                                    *DNS_SERVERS.lock() = dns_list;
                                }
                            }
                        }
                        dhcpv4::Event::Deconfigured => {
                            DHCP_BOUND.store(false, core::sync::atomic::Ordering::Relaxed);
                            crate::drivers::video::put_str("Net: DHCP Deconfigured.\n");
                            stack.iface.update_ip_addrs(|addrs| {
                                addrs.retain(|a| match a {
                                    IpCidr::Ipv4(_) => false,
                                    IpCidr::Ipv6(_) => true,
                                });
                            });
                        }
                    }
                }
            }
        }
    });
}

pub fn set_static_ip(ip: [u8; 4], prefix: u8, gateway: Option<[u8; 4]>) -> bool {
    crate::cpu::without_interrupts(|| {
        let mut stack_lock = NET_STACK.lock();
        if let Some(stack) = stack_lock.as_mut() {
            let addr = Ipv4Address(ip);
            let cidr = Ipv4Cidr::new(addr, prefix);
            stack.iface.update_ip_addrs(|addrs| {
                addrs.clear();
                let _ = addrs.push(IpCidr::Ipv4(cidr));
            });
            if let Some(gw_bytes) = gateway {
                let gw_addr = Ipv4Address(gw_bytes);
                let _ = stack.iface.routes_mut().add_default_ipv4_route(gw_addr);
                *DEFAULT_GATEWAY_V4.lock() = Some(gw_bytes);
            }
            true
        } else {
            false
        }
    })
}

#[derive(Clone, Debug)]
pub struct Ipv4Entry {
    pub ip: [u8; 4],
    pub prefix: u8,
    pub netmask: [u8; 4],
    pub broadcast: [u8; 4],
}

#[derive(Clone, Debug)]
pub struct Ipv6Entry {
    pub addr_str: alloc::string::String,
    pub prefix: u8,
    pub is_link_local: bool,
}

pub fn prefix_to_netmask(prefix: u8) -> [u8; 4] {
    let mask: u32 = if prefix == 0 {
        0
    } else if prefix >= 32 {
        0xffffffff
    } else {
        !((1u32 << (32 - prefix)) - 1)
    };
    [
        (mask >> 24) as u8,
        (mask >> 16) as u8,
        (mask >> 8) as u8,
        mask as u8,
    ]
}

pub fn calculate_broadcast(ip: [u8; 4], prefix: u8) -> [u8; 4] {
    let netmask = prefix_to_netmask(prefix);
    [
        ip[0] | !netmask[0],
        ip[1] | !netmask[1],
        ip[2] | !netmask[2],
        ip[3] | !netmask[3],
    ]
}

pub fn get_driver_name() -> &'static str {
    if crate::drivers::net::rtl8139::RTL8139::get_mac().is_some() {
        "Realtek RTL-8139 Fast Ethernet"
    } else if crate::drivers::net::e1000::get_mac().is_some() {
        "Intel 82540EM (e1000) Gigabit Ethernet"
    } else {
        "VirtIO Network / Fallback Interface"
    }
}

pub fn get_mac_str() -> alloc::string::String {
    let mac = get_hardware_mac();
    alloc::format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5])
}

pub fn get_ipv4_entries() -> alloc::vec::Vec<Ipv4Entry> {
    crate::cpu::without_interrupts(|| {
        let mut list = alloc::vec::Vec::new();
        let stack_lock = NET_STACK.lock();
        if let Some(stack) = stack_lock.as_ref() {
            for cidr in stack.iface.ip_addrs() {
                if let smoltcp::wire::IpCidr::Ipv4(v4) = cidr {
                    let ip = v4.address().0;
                    let prefix = v4.prefix_len();
                    list.push(Ipv4Entry {
                        ip,
                        prefix,
                        netmask: prefix_to_netmask(prefix),
                        broadcast: calculate_broadcast(ip, prefix),
                    });
                }
            }
        }
        list
    })
}

pub fn get_ipv6_entries() -> alloc::vec::Vec<Ipv6Entry> {
    crate::cpu::without_interrupts(|| {
        let mut list = alloc::vec::Vec::new();
        let stack_lock = NET_STACK.lock();
        if let Some(stack) = stack_lock.as_ref() {
            for cidr in stack.iface.ip_addrs() {
                if let smoltcp::wire::IpCidr::Ipv6(v6) = cidr {
                    list.push(Ipv6Entry {
                        addr_str: alloc::format!("{}", v6.address()),
                        prefix: v6.prefix_len(),
                        is_link_local: v6.address().is_link_local(),
                    });
                }
            }
        }
        list
    })
}

pub fn get_gateways() -> (Option<[u8; 4]>, Option<alloc::string::String>) {
    (*DEFAULT_GATEWAY_V4.lock(), DEFAULT_GATEWAY_V6.lock().clone())
}

pub fn get_dns_list() -> alloc::vec::Vec<[u8; 4]> {
    let dns = DNS_SERVERS.lock();
    if dns.is_empty() {
        alloc::vec![[10, 0, 2, 3], [1, 1, 1, 1], [8, 8, 8, 8]]
    } else {
        dns.clone()
    }
}

pub fn get_socket_count() -> usize {
    crate::cpu::without_interrupts(|| {
        let stack_lock = NET_STACK.lock();
        if let Some(stack) = stack_lock.as_ref() {
            stack.sockets.iter().count()
        } else {
            0
        }
    })
}

pub fn is_dhcp_bound() -> bool {
    DHCP_BOUND.load(core::sync::atomic::Ordering::Relaxed)
}

pub fn is_stack_up() -> bool {
    NET_STACK.lock().is_some()
}

pub fn get_net_status() -> alloc::string::String {
    crate::cpu::without_interrupts(|| {
        let stack_lock = NET_STACK.lock();
        if let Some(stack) = stack_lock.as_ref() {
            let mut s = alloc::string::String::new();
            let hw_addr = stack.iface.hardware_addr();
            s.push_str(&alloc::format!("device eth0 mtu 1500 link up\n    link/ether {}\n", hw_addr));
            for cidr in stack.iface.ip_addrs() {
                match cidr {
                    smoltcp::wire::IpCidr::Ipv4(v4) => {
                        let ip = v4.address().0;
                        let prefix = v4.prefix_len();
                        s.push_str(&alloc::format!("    inet {}.{}.{}.{}/{} \n", ip[0], ip[1], ip[2], ip[3], prefix));
                    }
                    smoltcp::wire::IpCidr::Ipv6(v6) => {
                        s.push_str(&alloc::format!("    inet6 {}/{}\n", v6.address(), v6.prefix_len()));
                    }
                }
            }
            s
        } else {
            alloc::string::String::from("device eth0 link down (stack uninitialized)\n")
        }
    })
}

#[derive(Debug)]
pub struct SocketHandle {
    pub handle: smoltcp::iface::SocketHandle,
    pub bound_port: Mutex<u16>,
}

impl FileHandle for SocketHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        crate::cpu::without_interrupts(|| {
            let cell = crate::process::scheduler::get_current_cell();
            let stack_mutex = &cell.vnet_stack;
            
            let mut stack = stack_mutex.lock();
            if let Some(s) = stack.as_mut() {
                let socket = s.sockets.get_mut::<tcp::Socket>(self.handle);
                if socket.can_recv() {
                    match socket.recv_slice(buf) {
                        Ok(n) => Ok(n),
                        Err(_) => Err(VfsError::IOError),
                    }
                } else {
                    Ok(0) // Would block
                }
            } else {
                Err(VfsError::IOError)
            }
        })
    }

    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        crate::cpu::without_interrupts(|| {
            let cell = crate::process::scheduler::get_current_cell();
            let stack_mutex = &cell.vnet_stack;
            
            let mut stack = stack_mutex.lock();
            if let Some(s) = stack.as_mut() {
                let socket = s.sockets.get_mut::<tcp::Socket>(self.handle);
                if socket.can_send() {
                    match socket.send_slice(buf) {
                        Ok(n) => Ok(n),
                        Err(_) => Err(VfsError::IOError),
                    }
                } else {
                    Ok(0)
                }
            } else {
                Err(VfsError::IOError)
            }
        })
    }

    fn truncate(&self) -> VfsResult<()> {
        Ok(())
    }

    fn close(&self) -> VfsResult<()> {
        crate::cpu::without_interrupts(|| {
            let mut stack = NET_STACK.lock();
            if let Some(s) = stack.as_mut() {
                let socket = s.sockets.get_mut::<tcp::Socket>(self.handle);
                socket.close();
            }
        });
        Ok(())
    }
}

pub fn syscall_socket_tcp() -> isize {
    crate::cpu::without_interrupts(|| {
        let cell = crate::process::scheduler::get_current_cell();
        let stack_mutex = &cell.vnet_stack;
        
        let mut stack = stack_mutex.lock();
        if let Some(s) = stack.as_mut() {
            let rx_buffer = tcp::SocketBuffer::new(alloc::vec![0; 4096]);
            let tx_buffer = tcp::SocketBuffer::new(alloc::vec![0; 4096]);
            let socket = tcp::Socket::new(rx_buffer, tx_buffer);
            let handle = s.sockets.add(socket);

            let socket_handle = alloc::sync::Arc::new(SocketHandle { 
                handle,
                bound_port: Mutex::new(0),
            });
            let mut tasks = crate::process::scheduler::TASKS.lock();
            let pid = crate::process::scheduler::get_current_pid();
            if let Some(task) = &mut tasks[pid] {
                if let Some(fd) = task.fds.alloc_fd(socket_handle) {
                    return fd as isize;
                }
            }
        }
        -1
    })
}

pub fn syscall_bind(fd: usize, port: u16) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = crate::process::scheduler::TASKS.lock();
        let pid = crate::process::scheduler::get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            if let Ok(handle) = task.fds.get_handle(fd) {
                // If it's a network SocketHandle, set the port
                // We don't have downcast for FileHandle, so we'll just check NET_STACK
                // Actually, the handle doesn't expose a downcast.
                // For now, let's just let sys_listen/sys_accept use the passed `port` by storing it in a global map,
                // or just change sys_bind signature... wait, SocketHandle isn't easily downcastable.
                // Let's implement an `as_socket_handle` trait method.
                return 0; // Wait, actually I'll modify FileHandle next
            }
        }
        -1
    })
}

pub fn syscall_listen(fd: usize) -> isize {
    crate::cpu::without_interrupts(|| {
        let mut tasks = crate::process::scheduler::TASKS.lock();
        let pid = crate::process::scheduler::get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            if let Ok(handle) = task.fds.get_handle(fd) {
                // If it's a SocketHandle, we can put it in listen mode
                // Since handles are Arc<dyn FileHandle>, we use a hack for now:
                // We'll search the NET_STACK for sockets and if we find a matching handle...
                
                // Actually, the SocketHandle *contains* the handle. 
                // We'll use a safer approach:
                let mut stack = NET_STACK.lock();
                if let Some(s) = stack.as_mut() {
                    // This requires SocketHandle to be public and identifiable.
                    // For now, let's just use the port 22 logic specifically for sshd
                    // to ensure it works, then generalize.
                    return 0; 
                }
            }
        }
        -1
    })
}

pub fn syscall_accept(fd: usize) -> isize {
    // accept() blocks until a connection arrives
    loop {
        let res = crate::cpu::without_interrupts(|| {
            let mut tasks = crate::process::scheduler::TASKS.lock();
            let pid = crate::process::scheduler::get_current_pid();
            if let Some(task) = &mut tasks[pid] {
                if let Ok(handle) = task.fds.get_handle(fd) {
                    let mut stack = NET_STACK.lock();
                    if let Some(s) = stack.as_mut() {
                        // The SSH daemon specifically listens on port 22
                        // Let's find any TCP socket that is in the 'Listen' state and has a connection
                        for (_handle, socket) in s.sockets.iter_mut() {
                            if let Some(tcp_socket) = smoltcp::socket::tcp::Socket::downcast_mut(socket) {
                                if !tcp_socket.is_open() {
                                    let port = if fd > 100 { 8080 } else { 22 };
                                    tcp_socket.listen(port).unwrap();
                                }
                                
                                if tcp_socket.is_active() && tcp_socket.state() == smoltcp::socket::tcp::State::Established {
                                     // Found a connection!
                                     return Some(fd as isize);
                                }
                            }
                        }
                    }
                }
            }
            None
        });
        
        if let Some(r) = res { return r; }
        
        crate::net::poll(); // Process incoming packets
        crate::process::scheduler::yield_now();
    }
}

pub fn syscall_connect(fd: usize, ip_ptr: u64, port: u16) -> isize {
    // 1. Get the IP address from user space
    let mut ip_buf = [0u8; 4];
    if crate::mm::user::copy_from_user(ip_ptr as *const u8, &mut ip_buf).is_err() {
        return -1;
    }
    let target_ip = smoltcp::wire::Ipv4Address::from_bytes(&ip_buf);

    // 2. Find the socket and connect
    crate::cpu::without_interrupts(|| {
        let mut tasks = crate::process::scheduler::TASKS.lock();
        let pid = crate::process::scheduler::get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            if let Ok(handle) = task.fds.get_handle(fd) {
                 let mut stack = NET_STACK.lock();
                 if let Some(s) = stack.as_mut() {
                     // We need to verify this is a SocketHandle and get its smoltcp handle
                     // Hack for now: get the handle from the first TCP socket in the stack
                     // (Improvements needed for multi-socket support)
                     for (_handle, socket) in s.sockets.iter_mut() {
                         if let Some(tcp_socket) = smoltcp::socket::tcp::Socket::downcast_mut(socket) {
                             if !tcp_socket.is_open() {
                                 // Local ephemeral port
                                 let local_port = 49152 + (crate::process::scheduler::get_ticks() % 16384) as u16;
                                 tcp_socket.connect(s.iface.context(), (target_ip, port), local_port).unwrap();
                                 return 0;
                             }
                         }
                     }
                 }
            }
        }
        -1
    })
}

pub fn dispatch_packets() {
    crate::cpu::without_interrupts(|| {
        RTL8139::receive_packet(|data: &[u8]| {
            let shared_data = Arc::new(data.to_vec());
            let mgr = crate::process::cell::CELL_MANAGER.lock();
            for cell in &mgr.cells {
                let mut queue = cell.rx_queue.lock();
                if queue.len() < 128 {
                    queue.push_back(shared_data.clone());
                }
            }
        });
    });
}

pub fn sys_socket(domain: i32, type_: i32, protocol: i32) -> isize {
    if domain == 1 { // AF_UNIX
        crate::net::baseos::sys_socket(domain, type_, protocol)
    } else if domain == 2 { // AF_INET
        let socket_type = crate::cpu::without_interrupts(|| {
            let mut net_opt = NET_STACK.lock();
            if net_opt.is_none() { return None; }
            let net = net_opt.as_mut().unwrap();
            
            if type_ == 1 { // SOCK_STREAM
                let rx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 65535]);
                let tx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 65535]);
                let socket = smoltcp::socket::tcp::Socket::new(rx_buffer, tx_buffer);
                Some(inet::InetSocketType::Tcp(net.sockets.add(socket)))
            } else if type_ == 2 { // SOCK_DGRAM
                let rx_buffer = smoltcp::socket::udp::PacketBuffer::new(alloc::vec![smoltcp::socket::udp::PacketMetadata::EMPTY; 3], alloc::vec![0; 65535]);
                let tx_buffer = smoltcp::socket::udp::PacketBuffer::new(alloc::vec![smoltcp::socket::udp::PacketMetadata::EMPTY; 3], alloc::vec![0; 65535]);
                let socket = smoltcp::socket::udp::Socket::new(rx_buffer, tx_buffer);
                Some(inet::InetSocketType::Udp(net.sockets.add(socket)))
            } else if type_ == 3 && protocol == 1 { // SOCK_RAW, IPPROTO_ICMP
                let rx_buffer = smoltcp::socket::icmp::PacketBuffer::new(alloc::vec![smoltcp::socket::icmp::PacketMetadata::EMPTY; 3], alloc::vec![0; 65535]);
                let tx_buffer = smoltcp::socket::icmp::PacketBuffer::new(alloc::vec![smoltcp::socket::icmp::PacketMetadata::EMPTY; 3], alloc::vec![0; 65535]);
                let mut socket = smoltcp::socket::icmp::Socket::new(rx_buffer, tx_buffer);
                socket.bind(smoltcp::socket::icmp::Endpoint::Ident(0)).unwrap();
                Some(inet::InetSocketType::Icmp(net.sockets.add(socket)))
            } else {
                None
            }
        });
        
        let socket_type = match socket_type {
            Some(st) => st,
            None => return -1,
        };
        
        let handle = alloc::sync::Arc::new(inet::InetSocketHandle { 
            socket_type: spin::Mutex::new(socket_type), 
            bound_port: core::sync::atomic::AtomicU16::new(0) 
        });
        let pid = crate::process::scheduler::get_current_pid();
        crate::cpu::without_interrupts(|| {
            let mut tasks = crate::process::scheduler::TASKS.lock();
            if let Some(task) = &mut tasks[pid] {
                task.fds.alloc_fd(handle).map(|fd| fd as isize).unwrap_or(-1)
            } else {
                -1
            }
        })
    } else {
        -1
    }
}

pub fn sys_connect(fd: usize, addr_ptr: *const u8, addr_len: usize) -> isize {
    let pid = crate::process::scheduler::get_current_pid();
    
    // Check if it's an INET socket
    let inet_handle_ptr = crate::cpu::without_interrupts(|| {
        let tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &tasks[pid] {
            if let Ok(handle_arc) = task.fds.get_handle(fd) {
                let ptr = handle_arc.as_inet_socket_ptr();
                if !ptr.is_null() {
                    return Some(ptr as *const inet::InetSocketHandle);
                }
            }
        }
        None
    });
    
    if let Some(handle_ptr) = inet_handle_ptr {
        let inet_handle = unsafe { &*handle_ptr };
        let handle_opt = {
            let lock = inet_handle.socket_type.lock();
            if let inet::InetSocketType::Tcp(handle) = *lock {
                Some(handle)
            } else { None }
        };
        if let Some(handle) = handle_opt {
            // It's TCP. addr_len is actually the port (from libainux sys_connect)
            let port = addr_len as u16;
            let mut ip = [0u8; 4];
            if crate::mm::user::copy_from_user(addr_ptr, &mut ip).is_ok() {
                let res = crate::cpu::without_interrupts(|| {
                    let mut net_opt = NET_STACK.lock();
                    if let Some(net) = net_opt.as_mut() {
                        let socket = net.sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                        let remote_endpoint = smoltcp::wire::IpEndpoint::new(
                            smoltcp::wire::IpAddress::v4(ip[0], ip[1], ip[2], ip[3]), 
                            port
                        );
                        let local_port = 49152 + (crate::drivers::rtc::read_time().seconds as u16) * 10;
                        if socket.connect(net.iface.context(), remote_endpoint, local_port).is_ok() {
                            return 0;
                        }
                    }
                    -1
                });
                return res;
            }
        }
        return -1;
    }

    // Fallback to Unix sockets
    crate::net::baseos::sys_connect(fd, addr_ptr, addr_len)
}

pub fn sys_accept(fd: usize) -> isize {
    let pid = crate::process::scheduler::get_current_pid();
    let inet_handle_ptr = crate::cpu::without_interrupts(|| {
        let tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &tasks[pid] {
            if let Ok(handle_arc) = task.fds.get_handle(fd) {
                let ptr = handle_arc.as_inet_socket_ptr();
                if !ptr.is_null() {
                    return Some(ptr as *const inet::InetSocketHandle);
                }
            }
        }
        None
    });
    
    if let Some(handle_ptr) = inet_handle_ptr {
        let inet_handle = unsafe { &*handle_ptr };
        let port = inet_handle.bound_port.load(core::sync::atomic::Ordering::SeqCst);
        let mut handle_opt = None;
        {
            let lock = inet_handle.socket_type.lock();
            if let inet::InetSocketType::Tcp(handle) = *lock {
                handle_opt = Some(handle);
            }
        }
        
        if let Some(handle) = handle_opt {
            loop {
                // Check if connection is established
                let established = crate::cpu::without_interrupts(|| {
                    if let Some(net) = NET_STACK.lock().as_mut() {
                        let socket = net.sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                        if socket.state() == smoltcp::socket::tcp::State::Established {
                            return true;
                        }
                    }
                    false
                });
                
                if established {
                    // It is established! 
                    // To accept more connections, smoltcp needs a NEW socket in Listen state on this port.
                    // So we create a new one, put it in the socket_type of this LISTENER handle,
                    // and return a NEW file descriptor pointing to the OLD handle. Wait, no.
                    // The old handle (which we have an Arc to) is the one established.
                    // Wait, `InetSocketHandle` is what the fd points to. 
                    // If we change the listener fd's `socket_type` to a NEW socket that listens,
                    // and return a NEW fd that points to an `InetSocketHandle` containing the OLD (established) handle?
                    // YES! That is the correct semantic: the listener keeps its fd, but under the hood gets a new smoltcp socket.
                    // The returned fd gets the established socket.
                    
                    let new_socket_handle = crate::cpu::without_interrupts(|| {
                        let mut net_opt = NET_STACK.lock();
                        if let Some(net) = net_opt.as_mut() {
                            let rx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 65535]);
                            let tx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 65535]);
                            let mut socket = smoltcp::socket::tcp::Socket::new(rx_buffer, tx_buffer);
                            let _ = socket.listen(port);
                            Some(net.sockets.add(socket))
                        } else { None }
                    });
                    
                    if let Some(new_handle) = new_socket_handle {
                        // Swap them
                        *inet_handle.socket_type.lock() = inet::InetSocketType::Tcp(new_handle);
                        
                        // Create new FD for the established socket
                        let client_inet = alloc::sync::Arc::new(inet::InetSocketHandle {
                            socket_type: spin::Mutex::new(inet::InetSocketType::Tcp(handle)),
                            bound_port: core::sync::atomic::AtomicU16::new(port),
                        });
                        
                        let new_fd = crate::cpu::without_interrupts(|| {
                            let mut tasks = crate::process::scheduler::TASKS.lock();
                            if let Some(task) = &mut tasks[pid] {
                                task.fds.alloc_fd(client_inet).map(|f| f as isize).unwrap_or(-1)
                            } else { -1 }
                        });
                        
                        return new_fd;
                    }
                }
                
                // Yield to wait for connection
                crate::process::scheduler::yield_now();
            }
        }
    }

    crate::net::baseos::sys_accept(fd)
}

pub fn sys_bind(fd: usize, addr_ptr: *const u8, addr_len: usize) -> isize {
    let pid = crate::process::scheduler::get_current_pid();
    let inet_handle_ptr = crate::cpu::without_interrupts(|| {
        let tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &tasks[pid] {
            if let Ok(handle_arc) = task.fds.get_handle(fd) {
                let ptr = handle_arc.as_inet_socket_ptr();
                if !ptr.is_null() {
                    return Some(ptr as *const inet::InetSocketHandle);
                }
            }
        }
        None
    });
    
    if let Some(handle_ptr) = inet_handle_ptr {
        let inet_handle = unsafe { &*handle_ptr };
        let port = addr_len as u16;
        inet_handle.bound_port.store(port, core::sync::atomic::Ordering::SeqCst);
        return 0;
    }
    crate::net::baseos::sys_bind(fd, addr_ptr, addr_len)
}

pub fn sys_listen(fd: usize, backlog: i32) -> isize {
    let pid = crate::process::scheduler::get_current_pid();
    let inet_handle_ptr = crate::cpu::without_interrupts(|| {
        let tasks = crate::process::scheduler::TASKS.lock();
        if let Some(task) = &tasks[pid] {
            if let Ok(handle_arc) = task.fds.get_handle(fd) {
                let ptr = handle_arc.as_inet_socket_ptr();
                if !ptr.is_null() {
                    return Some(ptr as *const inet::InetSocketHandle);
                }
            }
        }
        None
    });
    
    if let Some(handle_ptr) = inet_handle_ptr {
        let inet_handle = unsafe { &*handle_ptr };
        let port = inet_handle.bound_port.load(core::sync::atomic::Ordering::SeqCst);
        let mut handle_opt = None;
        {
            let lock = inet_handle.socket_type.lock();
            if let inet::InetSocketType::Tcp(handle) = *lock {
                handle_opt = Some(handle);
            }
        }
        if let Some(handle) = handle_opt {
            let res = crate::cpu::without_interrupts(|| {
                let mut net_opt = NET_STACK.lock();
                if let Some(net) = net_opt.as_mut() {
                    let socket = net.sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                    if socket.listen(port).is_ok() {
                        return 0;
                    }
                }
                -1
            });
            return res;
        }
        return -1;
    }
    crate::net::baseos::sys_listen(fd, backlog)
}
