use alloc::sync::Arc;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};
use smoltcp::socket::tcp::Socket as TcpSocket;
use smoltcp::socket::udp::Socket as UdpSocket;
use smoltcp::socket::icmp::Socket as IcmpSocket;

#[derive(Debug)]
pub enum InetSocketType {
    Tcp(smoltcp::iface::SocketHandle),
    Udp(smoltcp::iface::SocketHandle),
    Icmp(smoltcp::iface::SocketHandle),
}

pub struct InetSocketHandle {
    pub socket_type: spin::Mutex<InetSocketType>,
    pub bound_port: core::sync::atomic::AtomicU16,
}

impl core::fmt::Debug for InetSocketHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InetSocketHandle").finish()
    }
}

impl FileHandle for InetSocketHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        crate::cpu::without_interrupts(|| {
            let mut net_opt = crate::net::NET_STACK.lock();
            if net_opt.is_none() { return Err(VfsError::IOError); }
            let net = net_opt.as_mut().unwrap();
            
            match &*self.socket_type.lock() {
                InetSocketType::Tcp(handle) => {
                    let socket = net.sockets.get_mut::<TcpSocket>(*handle);
                    if socket.can_recv() {
                        match socket.recv_slice(buf) {
                            Ok(size) => Ok(size),
                            Err(_) => Err(VfsError::IOError),
                        }
                    } else if !socket.is_active() {
                        Ok(0) // Closed connection
                    } else {
                        Err(VfsError::IOError) // EAGAIN
                    }
                },
                InetSocketType::Udp(handle) => {
                    let socket = net.sockets.get_mut::<UdpSocket>(*handle);
                    if socket.can_recv() {
                        match socket.recv_slice(buf) {
                            Ok((size, _meta)) => Ok(size),
                            Err(_) => Err(VfsError::IOError),
                        }
                    } else {
                        Err(VfsError::IOError)
                    }
                },
                InetSocketType::Icmp(handle) => {
                    let socket = net.sockets.get_mut::<IcmpSocket>(*handle);
                    if socket.can_recv() {
                        match socket.recv_slice(buf) {
                            Ok((size, _ip)) => Ok(size),
                            Err(_) => Err(VfsError::IOError),
                        }
                    } else {
                        Err(VfsError::IOError)
                    }
                }
            }
        })
    }
    
    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        crate::cpu::without_interrupts(|| {
            let mut net_opt = crate::net::NET_STACK.lock();
            if net_opt.is_none() { return Err(VfsError::IOError); }
            let net = net_opt.as_mut().unwrap();
            
            match &*self.socket_type.lock() {
                InetSocketType::Tcp(handle) => {
                    let socket = net.sockets.get_mut::<TcpSocket>(*handle);
                    if socket.can_send() {
                        match socket.send_slice(buf) {
                            Ok(size) => Ok(size),
                            Err(_) => Err(VfsError::IOError),
                        }
                    } else {
                        Err(VfsError::IOError)
                    }
                },
                InetSocketType::Udp(handle) => {
                    let socket = net.sockets.get_mut::<UdpSocket>(*handle);
                    if socket.can_send() {
                        // For UDP, we assume localhost target if not specified via sendto
                        match socket.send_slice(buf, smoltcp::wire::IpEndpoint::new(smoltcp::wire::IpAddress::v4(127,0,0,1), 0)) {
                            Ok(()) => Ok(buf.len()),
                            Err(_) => Err(VfsError::IOError),
                        }
                    } else {
                        Err(VfsError::IOError)
                    }
                },
                InetSocketType::Icmp(handle) => {
                    let socket = net.sockets.get_mut::<IcmpSocket>(*handle);
                    if socket.can_send() {
                        // Hardcode target to 1.1.1.1 for ping.elf sys_write
                        let remote = smoltcp::wire::IpAddress::v4(1, 1, 1, 1);
                        socket.send_slice(buf, remote).unwrap_or(());
                        Ok(buf.len())
                    } else {
                        Err(VfsError::IOError)
                    }
                }
            }
        })
    }
    
    fn truncate(&self) -> VfsResult<()> {
        Err(VfsError::NotImplemented)
    }
    
    fn close(&self) -> VfsResult<()> {
        Ok(())
    }
    
    fn poll(&self, _events: u32) -> VfsResult<u32> {
        crate::cpu::without_interrupts(|| {
            let mut net_opt = crate::net::NET_STACK.lock();
            if net_opt.is_none() { return Ok(0); }
            let net = net_opt.as_mut().unwrap();
            
            let mut events = 0;
            match &*self.socket_type.lock() {
                InetSocketType::Tcp(handle) => {
                    let socket = net.sockets.get_mut::<TcpSocket>(*handle);
                    if socket.can_recv() { events |= 1; /* POLLIN */ }
                    if socket.can_send() { events |= 4; /* POLLOUT */ }
                },
                InetSocketType::Udp(handle) => {
                    let socket = net.sockets.get_mut::<UdpSocket>(*handle);
                    if socket.can_recv() { events |= 1; }
                    if socket.can_send() { events |= 4; }
                },
                InetSocketType::Icmp(handle) => {
                    let socket = net.sockets.get_mut::<IcmpSocket>(*handle);
                    if socket.can_recv() { events |= 1; }
                    if socket.can_send() { events |= 4; }
                }
            }
            Ok(events)
        })
    }
    
    fn as_inet_socket_ptr(&self) -> *const () {
        self as *const _ as *const ()
    }
}

pub fn tcp_connect(ip: [u8; 4], port: u16) -> Option<InetSocketHandle> {
    crate::cpu::without_interrupts(|| {
        let mut net_opt = crate::net::NET_STACK.lock();
        if net_opt.is_none() { return None; }
        let net = net_opt.as_mut().unwrap();

        let rx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 4096]);
        let tx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 4096]);
        let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
        
        let remote_endpoint = smoltcp::wire::IpEndpoint::new(
            smoltcp::wire::IpAddress::v4(ip[0], ip[1], ip[2], ip[3]), 
            port
        );
        let local_port = 49152 + (crate::drivers::rtc::read_time().seconds as u16) * 10; // Simple ephemeral port

        if socket.connect(net.iface.context(), remote_endpoint, local_port).is_ok() {
            let handle = net.sockets.add(socket);
            Some(InetSocketHandle {
                socket_type: spin::Mutex::new(InetSocketType::Tcp(handle)),
                bound_port: core::sync::atomic::AtomicU16::new(local_port),
            })
        } else {
            None
        }
    })
}

pub fn tcp_listen(port: u16) -> Option<InetSocketHandle> {
    crate::cpu::without_interrupts(|| {
        let mut net_opt = crate::net::NET_STACK.lock();
        if net_opt.is_none() { return None; }
        let net = net_opt.as_mut().unwrap();

        let rx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 4096]);
        let tx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 4096]);
        let mut socket = TcpSocket::new(rx_buffer, tx_buffer);

        if socket.listen(port).is_ok() {
            let handle = net.sockets.add(socket);
            Some(InetSocketHandle {
                socket_type: spin::Mutex::new(InetSocketType::Tcp(handle)),
                bound_port: core::sync::atomic::AtomicU16::new(port),
            })
        } else {
            None
        }
    })
}
