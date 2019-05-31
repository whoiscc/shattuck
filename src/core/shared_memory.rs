//

use crate::core::inc::Inc;
use crate::core::runtime_error::RuntimeError;

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

struct CountedObject<O> {
    object: RwLock<O>,
    count: usize,
}

struct SharedMemoryPriv<O> {
    objects: HashMap<usize, CountedObject<O>>,
    object_id: Inc,
    max_count: usize,
}

impl<O> SharedMemoryPriv<O> {
    fn new(count: usize) -> Self {
        Self {
            objects: HashMap::new(),
            object_id: Inc::new(),
            max_count: count,
        }
    }

    fn insert(&mut self, object: O) -> Result<usize, RuntimeError> {
        if self.objects.len() == self.max_count {
            return Err(RuntimeError::MemoryFull);
        }
        let object_id = self.object_id.create();
        self.objects.insert(
            object_id,
            CountedObject {
                object: RwLock::new(object),
                count: 0,
            },
        );
        Ok(object_id)
    }

    fn hold(&mut self, object_id: usize) -> Result<(), RuntimeError> {
        self.objects
            .get_mut(&object_id)
            .ok_or(RuntimeError::SegFault)?
            .count += 1;
        Ok(())
    }

    fn unhold(&mut self, object_id: usize) -> Result<(), RuntimeError> {
        let count = &mut self
            .objects
            .get_mut(&object_id)
            .ok_or(RuntimeError::SegFault)?
            .count;
        *count -= 1;
        if *count == 0 {
            self.objects.remove(&object_id);
        }
        Ok(())
    }
}

pub struct SharedMemory<O> {
    internal: Arc<RwLock<SharedMemoryPriv<O>>>,
}

impl<O> SharedMemory<O> {
    pub fn new(count: usize) -> Self {
        Self {
            internal: Arc::new(RwLock::new(SharedMemoryPriv::new(count))),
        }
    }

    pub fn insert(&self, object: O) -> Result<usize, RuntimeError> {
        self.internal.write().unwrap().insert(object)
    }

    pub fn distribute(&self, object_id: usize) -> Result<RemoteObject<O>, RuntimeError> {
        let internal = Arc::clone(&self.internal);
        internal.write().unwrap().hold(object_id)?;
        Ok(RemoteObject {
            internal,
            object_id,
        })
    }
}

#[derive(Clone)]
pub struct RemoteObject<O> {
    internal: Arc<RwLock<SharedMemoryPriv<O>>>,
    object_id: usize,
}

pub struct RemoteObjectGuard<'a, O> {
    guard: RwLockReadGuard<'a, SharedMemoryPriv<O>>,
    object_id: usize,
}

impl<O> RemoteObject<O> {
    pub fn get(&self) -> RemoteObjectGuard<O> {
        RemoteObjectGuard {
            guard: self.internal.read().unwrap(),
            object_id: self.object_id,
        }
    }
}

impl<O> Drop for RemoteObject<O> {
    fn drop(&mut self) {
        self.internal
            .write()
            .unwrap()
            .unhold(self.object_id)
            .expect("segfault");
    }
}

pub struct ReadRemoteObject<'a, O> {
    guard: RwLockReadGuard<'a, O>,
}

pub struct WriteRemoteObject<'a, O> {
    guard: RwLockWriteGuard<'a, O>,
}

impl<'a, O> RemoteObjectGuard<'a, O> {
    pub fn read(&self) -> Result<ReadRemoteObject<O>, RuntimeError> {
        let read = ReadRemoteObject {
            guard: self
                .guard
                .objects
                .get(&self.object_id)
                .expect("segfault")
                .object
                .try_read()
                .map_err(|_| RuntimeError::AccessConflict)?,
        };
        Ok(read)
    }

    pub fn write(&self) -> Result<WriteRemoteObject<O>, RuntimeError> {
        let write = WriteRemoteObject {
            guard: self
                .guard
                .objects
                .get(&self.object_id)
                .expect("segfault")
                .object
                .write()
                .map_err(|_| RuntimeError::AccessConflict)?,
        };
        Ok(write)
    }
}

impl<'a, O> Deref for ReadRemoteObject<'a, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, O> Deref for WriteRemoteObject<'a, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, O> DerefMut for WriteRemoteObject<'a, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq, Eq, Debug)]
    struct Object(i64);

    #[test]
    fn test_share_read() {
        let shared = SharedMemory::<Object>::new(16);
        let obj_id = shared.insert(Object(42)).unwrap();
        let remote1 = shared.distribute(obj_id).unwrap();
        let remote2 = shared.distribute(obj_id).unwrap();
        assert_eq!(&remote1.get().read().unwrap() as &Object, &Object(42));
        assert_eq!(&remote2.get().read().unwrap() as &Object, &Object(42));
    }

    #[test]
    fn test_share_read_write() {
        let shared = SharedMemory::<Object>::new(16);
        let obj_id = shared.insert(Object(42)).unwrap();
        let remote1 = shared.distribute(obj_id).unwrap();
        let remote2 = shared.distribute(obj_id).unwrap();
        *(&mut remote1.get().write().unwrap().0) = 43;
        assert_eq!(&remote2.get().read().unwrap() as &Object, &Object(43));
    }

    #[test]
    fn test_collect() {
        let shared = SharedMemory::<Object>::new(16);
        let obj_id = shared.insert(Object(42)).unwrap();
        {
            let _remote = shared.distribute(obj_id).unwrap();
        }
        assert!(shared.distribute(obj_id).is_err());
    }
}
