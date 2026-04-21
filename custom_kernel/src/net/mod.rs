extern crate alloc;
use alloc::vec::Vec;
use spin::Mutex;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr};
use smoltcp::socket::{dhcpv4, tcp};
use crate::drivers::net::rtl8139::RTL8139;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};

pub struct AinuxDevice;

impl Device for AinuxDevice {
    type RxToken<'a> = AinuxRxToken;
    type TxToken<'a> = AinuxTxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut result = None;
        RTL8139::receive_packet(|data| {
            result = Some((AinuxRxToken { buffer: data.to_vec() }, AinuxTxToken));
        });
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
        result
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
    let mut caps = DeviceCapabilities::default();
    caps.max_transmission_unit = 1500;
    caps.medium = Medium::Ethernet;

    let mut device = AinuxDevice;
    let config = Config::new(EthernetAddress([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]).into());
    
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
        let mut stack_lock = NET_STACK.lock();
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
                            crate::drivers::video::put_str(&alloc::format!("Net: DHCP Configured! IP: {}\n", config.address));
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
}

impl FileHandle for SocketHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        crate::cpu::without_interrupts(|| {
            let mut stack = NET_STACK.lock();
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
            let mut stack = NET_STACK.lock();
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
        let mut stack = NET_STACK.lock();
        if let Some(s) = stack.as_mut() {
            let rx_buffer = tcp::SocketBuffer::new(alloc::vec![0; 4096]);
            let tx_buffer = tcp::SocketBuffer::new(alloc::vec![0; 4096]);
            let socket = tcp::Socket::new(rx_buffer, tx_buffer);
            let handle = s.sockets.add(socket);

            let socket_handle = alloc::sync::Arc::new(SocketHandle { handle });
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
            if let Some(_entry) = task.fds.get_entry(fd) {
                let mut stack = NET_STACK.lock();
                if let Some(_s) = stack.as_mut() {
                    return 0;
                }
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
            if let Some(_file_desc) = task.fds.get_entry(fd) {
                return 0;
            }
        }
        -1
    })
}

pub fn syscall_accept(fd: usize) -> isize {
    loop {
        let res = crate::cpu::without_interrupts(|| {
            let mut tasks = crate::process::scheduler::TASKS.lock();
            let pid = crate::process::scheduler::get_current_pid();
            if let Some(_task) = &mut tasks[pid] {
                // ... logic to check for connections ...
                return Some(0); // Stub
            }
            Some(-1)
        });
        if let Some(r) = res { return r; }
        crate::process::scheduler::yield_now();
    }
}
