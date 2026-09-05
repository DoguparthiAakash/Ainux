use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub enum IOValue {
    Integer(i64),
    String(String),
    Boolean(bool),
    Data(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum IOError {
    NotFound,
    NoMemory,
    DeviceError,
    InvalidArgs,
    Unsupported,
}

pub type IOResult<T> = Result<T, IOError>;
