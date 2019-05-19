//

extern crate crossbeam;
use crossbeam::sync::{ShardedLock, ShardedLockReadGuard, ShardedLockWriteGuard};

use crate::core::runtime::{Pointer, Runtime, RuntimeError};

pub struct SharedRuntime(ShardedLock<Runtime>);
pub type ReadRuntime<'a> = ShardedLockReadGuard<'a, Runtime>;
pub type WriteRuntime<'a> = ShardedLockWriteGuard<'a, Runtime>;

impl SharedRuntime {
    pub fn new(max_object_count: usize) -> Result<Self, RuntimeError> {
        let shared = SharedRuntime(ShardedLock::new(Runtime::new(max_object_count)?));
        Ok(shared)
    }

    pub fn read(&self) -> ReadRuntime {
        let Self(lock) = self;
        lock.read().unwrap()
    }

    pub fn write(&self) -> WriteRuntime {
        let Self(lock) = self;
        lock.write().unwrap()
    }

    pub fn run(&self, method: Pointer) -> Result<(), RuntimeError> {
        Runtime::run_method(self, method)
    }
}

pub fn with(shared: &SharedRuntime) -> ReadRuntime {
    shared.read()
}

pub fn with_mut(shared: &SharedRuntime) -> WriteRuntime {
    shared.write()
}
