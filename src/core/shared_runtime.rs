//

extern crate crossbeam;
use crossbeam::sync::{ShardedLock, ShardedLockReadGuard, ShardedLockWriteGuard};

use std::sync::TryLockError;

use crate::core::runtime::{Pointer, Runtime, RuntimeError};

pub struct SharedRuntime(ShardedLock<Runtime>);
pub type ReadRuntime<'a> = ShardedLockReadGuard<'a, Runtime>;
pub type WriteRuntime<'a> = ShardedLockWriteGuard<'a, Runtime>;

#[derive(Debug)]
pub enum SharedRuntimeError {
    WouldBlock,
}

impl<T> From<TryLockError<T>> for SharedRuntimeError {
    fn from(err: TryLockError<T>) -> Self {
        match err {
            TryLockError::WouldBlock => SharedRuntimeError::WouldBlock,
            _ => panic!(),
        }
    }
}

impl SharedRuntime {
    pub fn new(max_object_count: usize) -> Result<Self, RuntimeError> {
        let shared = SharedRuntime(ShardedLock::new(Runtime::new(max_object_count)?));
        Ok(shared)
    }

    pub fn read(&self) -> Result<ReadRuntime, SharedRuntimeError> {
        let Self(lock) = self;
        lock.try_read().map_err(TryLockError::into)
    }

    pub fn write(&self) -> Result<WriteRuntime, SharedRuntimeError> {
        let Self(lock) = self;
        lock.try_write().map_err(TryLockError::into)
    }

    pub fn run(&self, method: Pointer) -> Result<(), RuntimeError> {
        Runtime::run_method(self, method)
    }
}

pub fn with(shared: &SharedRuntime) -> Result<ReadRuntime, SharedRuntimeError> {
    shared.read()
}

pub fn with_mut(shared: &SharedRuntime) -> Result<WriteRuntime, SharedRuntimeError> {
    shared.write()
}
