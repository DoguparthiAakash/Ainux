use alloc::string::String;
use alloc::vec::Vec;

/// 9P/Styx Message Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Tversion = 100, Rversion = 101,
    Tauth = 102,    Rauth = 103,
    Tattach = 104,  Rattach = 105,
    Terror = 106,   Rerror = 107,
    Tflush = 108,   Rflush = 109,
    Twalk = 110,    Rwalk = 111,
    Topen = 112,    Ropen = 113,
    Tcreate = 114,  Rcreate = 115,
    Tread = 116,    Rread = 117,
    Twrite = 118,   Rwrite = 119,
    Tclunk = 120,   Rclunk = 121,
    Tremove = 122,  Rremove = 123,
    Tstat = 124,    Rstat = 125,
    Twstat = 126,   Rwstat = 127,
}

impl MessageType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            100 => Some(Self::Tversion), 101 => Some(Self::Rversion),
            102 => Some(Self::Tauth),    103 => Some(Self::Rauth),
            104 => Some(Self::Tattach),  105 => Some(Self::Rattach),
            106 => Some(Self::Terror),   107 => Some(Self::Rerror),
            108 => Some(Self::Tflush),   109 => Some(Self::Rflush),
            110 => Some(Self::Twalk),    111 => Some(Self::Rwalk),
            112 => Some(Self::Topen),    113 => Some(Self::Ropen),
            114 => Some(Self::Tcreate),  115 => Some(Self::Rcreate),
            116 => Some(Self::Tread),    117 => Some(Self::Rread),
            118 => Some(Self::Twrite),   119 => Some(Self::Rwrite),
            120 => Some(Self::Tclunk),   121 => Some(Self::Rclunk),
            122 => Some(Self::Tremove),  123 => Some(Self::Rremove),
            124 => Some(Self::Tstat),    125 => Some(Self::Rstat),
            126 => Some(Self::Twstat),   127 => Some(Self::Rwstat),
            _ => None,
        }
    }
}

/// 9P/Styx Qid (Unique File Identifier)
#[derive(Debug, Clone, Copy)]
pub struct Qid {
    pub file_type: u8,
    pub version: u32,
    pub path: u64,
}

/// 9P/Styx Message Frame
#[derive(Debug, Clone)]
pub struct StyxMessage {
    pub size: u32,
    pub m_type: MessageType,
    pub tag: u16,
    pub payload: Vec<u8>,
}

impl StyxMessage {
    pub fn new(m_type: MessageType, tag: u16, payload: Vec<u8>) -> Self {
        let size = 4 + 1 + 2 + payload.len() as u32; // size(4) + type(1) + tag(2) + payload
        Self {
            size,
            m_type,
            tag,
            payload,
        }
    }
}

/// Styx Server Trait (Can be implemented by FS or IPC endpoints)
pub trait StyxServer {
    fn handle_message(&mut self, msg: &StyxMessage) -> StyxMessage;
}

/// Inferno OS inspired Per-Process Namespace
#[derive(Debug, Clone)]
pub struct Namespace {
    // Maps local paths to 9P Channel IDs or local handlers
    pub mounts: Vec<(String, u32)>, 
}

impl Namespace {
    pub fn new() -> Self {
        Self { mounts: Vec::new() }
    }
}
