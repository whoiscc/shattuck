//

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub enum RuntimeError {
    SegFault,
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            RuntimeError::SegFault => write!(f, "segfault"),
        }
    }
}

impl Error for RuntimeError {}
