use std::error;
use std::fmt;

use super::op::Op;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    BadInstruction,
    DataAbort,
    DriverMissing,
    LoadFailure,
    MalformedOp(Op),
    PrefetchAbort,
    StackOverflow,
    StackUnderflow,
    UnimplementedOp(Op),
}

pub type Result<T> = std::result::Result<T, Error>;

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Error {
    pub fn fatal(&self) -> bool {
        match *self {
            Error::DriverMissing => false,
            Error::MalformedOp(_) => false,
            Error::UnimplementedOp(_) => false,
            _ => true,
        }
    }
}
