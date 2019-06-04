//

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub enum RuntimeError {
    SegFault,
    AccessConflict,
    MemoryFull,
    NotCallable,
    ContextNotSet,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            RuntimeError::SegFault => write!(f, "segfault"),
            RuntimeError::AccessConflict => write!(f, "access conflict"),
            RuntimeError::MemoryFull => write!(f, "memory full"),
            RuntimeError::NotCallable => write!(f, "not callable"),
            RuntimeError::ContextNotSet => write!(f, "context not set"),
        }
    }
}

impl Error for RuntimeError {}
