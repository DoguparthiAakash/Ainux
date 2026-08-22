extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr};
use smoltcp::socket::{dhcpv4, tcp};
use crate::drivers::net::rtl8139::RTL8139;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};
use smoltcp::socket::AnySocket;

pub mod dns;
pub mod wifi_80211;
pub mod unix;
pub mod inet;

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

pub fn init() {
    let mut caps = smoltcp::phy::DeviceCapabilities::default();
    caps.max_transmission_unit = 1500;
    caps.medium = smoltcp::phy::Medium::Ethernet;

    let mut device = AinuxDevice;
    let config = smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(smoltcp::wire::EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56])));
    
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));
    let mut sockets = SocketSet::new(Vec::new());

    // Add DHCP socket
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

    crate::drivers::video::put_str("Net: smoltcp Stack Initialized (DHCP Active).\n");
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
                            crate::drivers::video::put_str(&alloc::format!("Net: Cell DHCP Configured! IP: {}\n", config.address));
                            stack.iface.update_ip_addrs(|addrs| {
                                addrs.push(IpCidr::Ipv4(config.address)).unwrap();
                            });
                            if let Some(router) = config.router {
                                stack.iface.routes_mut().add_default_ipv4_route(router).unwrap();
                            }
                        }
                        dhcpv4::Event::Deconfigured => {
                            crate::drivers::video::put_str("Net: DHCP Deconfigured.\n");
                            stack.iface.update_ip_addrs(|addrs| {
                                addrs.clear();
                            });
                        }
                    }
                }
            }
        }
    });
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
                return 0;
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
        crate::net::unix::sys_socket(domain, type_, protocol)
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
    crate::net::unix::sys_connect(fd, addr_ptr, addr_len)
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

    crate::net::unix::sys_accept(fd)
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
    crate::net::unix::sys_bind(fd, addr_ptr, addr_len)
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
    crate::net::unix::sys_listen(fd, backlog)
}
