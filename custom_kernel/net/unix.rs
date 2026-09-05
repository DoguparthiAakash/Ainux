use alloc::string::String;
use alloc::string::ToString;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use spin::Mutex;
use crate::fs::vfs::{FileHandle, VfsResult, VfsError};
use crate::process::scheduler;

lazy_static::lazy_static! {
    pub static ref UNIX_SOCKET_REGISTRY: Mutex<BTreeMap<String, Arc<UnixServer>>> = Mutex::new(BTreeMap::new());
}

#[derive(Debug)]
pub struct UnixServer {
    pub path: String,
    // Accept queue holds the B-side of a new connection. 
    // The client takes the A-side.
    pub accept_queue: Mutex<VecDeque<Arc<UnixSocketHandle>>>,
}

#[derive(Debug)]
pub struct UnixConnection {
    pub a_to_b: Mutex<VecDeque<u8>>,
    pub b_to_a: Mutex<VecDeque<u8>>,
    pub fds_a_to_b: Mutex<VecDeque<i32>>,
    pub fds_b_to_a: Mutex<VecDeque<i32>>,
}

impl UnixConnection {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            a_to_b: Mutex::new(VecDeque::new()),
            b_to_a: Mutex::new(VecDeque::new()),
            fds_a_to_b: Mutex::new(VecDeque::new()),
            fds_b_to_a: Mutex::new(VecDeque::new()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketSide {
    SideA,
    SideB,
}

#[derive(Debug)]
pub enum UnixSocketState {
    Unbound,
    Server(Arc<UnixServer>),
    Connected(Arc<UnixConnection>, SocketSide),
}

#[derive(Debug)]
pub struct UnixSocketHandle {
    pub state: Mutex<UnixSocketState>,
}

impl FileHandle for UnixSocketHandle {
    fn read(&self, buf: &mut [u8], _offset: u64) -> VfsResult<usize> {
        let state = self.state.lock();
        if let UnixSocketState::Connected(conn, side) = &*state {
            let mut rx = if *side == SocketSide::SideA {
                conn.b_to_a.lock()
            } else {
                conn.a_to_b.lock()
            };
            
            if rx.is_empty() {
                // Non-blocking for now
                return Ok(0);
            }
            
            let mut read_bytes = 0;
            for b in buf.iter_mut() {
                if let Some(data) = rx.pop_front() {
                    *b = data;
                    read_bytes += 1;
                } else {
                    break;
                }
            }
            return Ok(read_bytes);
        }
        Err(VfsError::IOError)
    }

    fn write(&self, buf: &[u8], _offset: u64) -> VfsResult<usize> {
        let state = self.state.lock();
        if let UnixSocketState::Connected(conn, side) = &*state {
            let mut tx = if *side == SocketSide::SideA {
                conn.a_to_b.lock()
            } else {
                conn.b_to_a.lock()
            };
            
            for &b in buf {
                tx.push_back(b);
            }
            return Ok(buf.len());
        }
        Err(VfsError::IOError)
    }

    fn truncate(&self) -> VfsResult<()> {
        Ok(())
    }

    fn close(&self) -> VfsResult<()> {
        let mut state = self.state.lock();
        if let UnixSocketState::Server(server) = &*state {
            UNIX_SOCKET_REGISTRY.lock().remove(&server.path);
        }
        *state = UnixSocketState::Unbound;
        Ok(())
    }
    
    fn as_unix_socket_ptr(&self) -> *const () {
        self as *const _ as *const ()
    }
}

pub fn sys_socket(domain: i32, _type: i32, _protocol: i32) -> isize {
    if domain != 1 { return -1; } // AF_UNIX only
    
    let handle = Arc::new(UnixSocketHandle {
        state: Mutex::new(UnixSocketState::Unbound),
    });
    
    let mut tasks = scheduler::TASKS.lock();
    let pid = scheduler::get_current_pid();
    if let Some(task) = &mut tasks[pid] {
        if let Some(fd) = task.fds.alloc_fd(handle) {
            return fd as isize;
        }
    }
    -1
}

pub fn sys_bind(fd: usize, path_ptr: *const u8, len: usize) -> isize {
    let mut path_buf = alloc::vec![0u8; len];
    if crate::mm::user::copy_from_user(path_ptr, &mut path_buf).is_err() {
        return -1;
    }
    let path = String::from_utf8_lossy(&path_buf).into_owned();
    let path = path.trim_end_matches('\0').to_string();
    
    let mut tasks = scheduler::TASKS.lock();
    let pid = scheduler::get_current_pid();
    if let Some(task) = &mut tasks[pid] {
        if let Ok(handle_arc) = task.fds.get_handle(fd) {
            let ptr = handle_arc.as_unix_socket_ptr();
            if !ptr.is_null() {
                let unix_handle = unsafe { &*(ptr as *const UnixSocketHandle) };
                let mut state = unix_handle.state.lock();
                if let UnixSocketState::Unbound = &*state {
                    let server = Arc::new(UnixServer {
                        path: path.clone(),
                        accept_queue: Mutex::new(VecDeque::new()),
                    });
                    UNIX_SOCKET_REGISTRY.lock().insert(path, server.clone());
                    *state = UnixSocketState::Server(server);
                    return 0;
                }
            }
        }
    }
    -1
}

pub fn sys_listen(fd: usize, _backlog: i32) -> isize {
    // ALready listening in bind
    0
}

pub fn sys_connect(fd: usize, path_ptr: *const u8, len: usize) -> isize {
    let mut path_buf = alloc::vec![0u8; len];
    if crate::mm::user::copy_from_user(path_ptr, &mut path_buf).is_err() {
        return -1;
    }
    let path = String::from_utf8_lossy(&path_buf).into_owned();
    let path = path.trim_end_matches('\0').to_string();
    
    let registry = UNIX_SOCKET_REGISTRY.lock();
    if let Some(server) = registry.get(&path) {
        let conn = UnixConnection::new();
        
        let client_side = UnixSocketState::Connected(conn.clone(), SocketSide::SideA);
        let server_side = Arc::new(UnixSocketHandle {
            state: Mutex::new(UnixSocketState::Connected(conn, SocketSide::SideB)),
        });
        
        server.accept_queue.lock().push_back(server_side);
        
        let mut tasks = scheduler::TASKS.lock();
        let pid = scheduler::get_current_pid();
        if let Some(task) = &mut tasks[pid] {
            if let Ok(handle_arc) = task.fds.get_handle(fd) {
                let ptr = handle_arc.as_unix_socket_ptr();
                if !ptr.is_null() {
                    let unix_handle = unsafe { &*(ptr as *const UnixSocketHandle) };
                    let mut state = unix_handle.state.lock();
                    *state = client_side;
                    return 0;
                }
            }
        }
    }
    -1
}

pub fn sys_accept(fd: usize) -> isize {
    let mut tasks = scheduler::TASKS.lock();
    let pid = scheduler::get_current_pid();
    if let Some(task) = &mut tasks[pid] {
        if let Ok(handle_arc) = task.fds.get_handle(fd) {
            let ptr = handle_arc.as_unix_socket_ptr();
            if !ptr.is_null() {
                let unix_handle = unsafe { &*(ptr as *const UnixSocketHandle) };
                let state = unix_handle.state.lock();
                if let UnixSocketState::Server(server) = &*state {
                    let mut queue = server.accept_queue.lock();
                    if let Some(server_side_handle) = queue.pop_front() {
                        if let Some(new_fd) = task.fds.alloc_fd(server_side_handle) {
                            return new_fd as isize;
                        }
                    } else {
                        // Queue empty, would block
                        return -1; // EAGAIN
                    }
                }
            }
        }
    }
    -1
}
