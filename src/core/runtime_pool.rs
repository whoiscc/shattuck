//

use std::cell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut};

extern crate crossbeam;
use crossbeam::sync::ShardedLock;

pub type SharedRuntimes = ShardedLock<RuntimePool>;

use crate::core::runtime::{Runtime, RuntimeError};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct RuntimeId(usize);

pub struct RuntimePool {
    runtimes: HashMap<RuntimeId, RefCell<Runtime>>,
    next_id: usize,
}

#[derive(Debug)]
pub enum RuntimePoolError {
    FailToCreateRuntime(RuntimeError),
    FailToBorrow(RuntimeId, BorrowError),
    FailToBorrowMut(RuntimeId, BorrowMutError),
    InvalidRuntimeId(RuntimeId),
}

impl Display for RuntimePoolError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            RuntimePoolError::FailToCreateRuntime(runtime_error) => {
                write!(f, "fail to create runtime: {}", runtime_error)
            }
            RuntimePoolError::FailToBorrow(runtime_id, borrow_error) => {
                write!(f, "fail to borrow {:?}: {}", runtime_id, borrow_error)
            }
            RuntimePoolError::FailToBorrowMut(runtime_id, borrow_mut_error) => write!(
                f,
                "fail to borrow mut {:?}: {}",
                runtime_id, borrow_mut_error
            ),
            RuntimePoolError::InvalidRuntimeId(runtime_id) => {
                write!(f, "invalid runtime id {:?}", runtime_id)
            }
        }
    }
}

impl Error for RuntimePoolError {}

impl RuntimePool {
    pub fn new_shared() -> SharedRuntimes {
        SharedRuntimes::new(RuntimePool {
            runtimes: HashMap::new(),
            next_id: 0,
        })
    }

    pub fn create_runtime(
        &mut self,
        max_object_count: usize,
    ) -> Result<RuntimeId, RuntimePoolError> {
        let runtime_id = RuntimeId(self.next_id);
        self.next_id += 1;
        self.runtimes.insert(
            runtime_id,
            RefCell::new(
                Runtime::new(max_object_count).map_err(|runtime_error| {
                    RuntimePoolError::FailToCreateRuntime(runtime_error)
                })?,
            ),
        );
        Ok(runtime_id)
    }

    pub fn borrow(&self, runtime_id: RuntimeId) -> Result<Ref<Runtime>, RuntimePoolError> {
        let runtime_cell = self
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| RuntimePoolError::InvalidRuntimeId(runtime_id))?;
        let borrowed_runtime = runtime_cell
            .try_borrow()
            .map_err(|borrow_error| RuntimePoolError::FailToBorrow(runtime_id, borrow_error))?;
        Ok(borrowed_runtime)
    }

    pub fn borrow_mut(&self, runtime_id: RuntimeId) -> Result<RefMut<Runtime>, RuntimePoolError> {
        let runtime_cell = self
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| RuntimePoolError::InvalidRuntimeId(runtime_id))?;
        let borrowed_runtime = runtime_cell.try_borrow_mut().map_err(|borrow_mut_error| {
            RuntimePoolError::FailToBorrowMut(runtime_id, borrow_mut_error)
        })?;
        Ok(borrowed_runtime)
    }
}
