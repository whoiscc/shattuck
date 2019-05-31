//

use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::memory::Memory;
use crate::core::runtime_error::RuntimeError;

enum QuasiObject<O, S> {
    Local(O),
    Remote(Arc<RwLock<S>>),
    Temp,
}

pub enum ReadObject<'a, O, S> {
    Local(&'a O),
    Remote(RwLockReadGuard<'a, S>),
}

impl<'a, O, S> Deref for ReadObject<'a, O, S>
where
    S: Deref<Target = O>,
{
    type Target = O;

    fn deref(&self) -> &Self::Target {
        match self {
            ReadObject::Local(object) => object,
            ReadObject::Remote(guard) => guard,
        }
    }
}

pub enum WriteObject<'a, O, S> {
    Local(&'a mut O),
    Remote(RwLockWriteGuard<'a, S>),
}

impl<'a, O, S> Deref for WriteObject<'a, O, S>
where
    S: Deref<Target = O>,
{
    type Target = O;

    fn deref(&self) -> &Self::Target {
        match self {
            WriteObject::Local(object) => object,
            WriteObject::Remote(guard) => guard,
        }
    }
}

impl<'a, O, S> DerefMut for WriteObject<'a, O, S>
where
    S: DerefMut<Target = O>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            WriteObject::Local(object) => object,
            WriteObject::Remote(guard) => guard,
        }
    }
}

pub struct ShareObject<S>(Arc<RwLock<S>>);

pub struct Runtime<O, S> {
    memory: Memory<QuasiObject<O, S>>,
}

impl<O, S> Runtime<O, S>
where
    S: Sync + From<O> + DerefMut<Target = O>,
{
    pub fn new(count: usize) -> Self {
        Self {
            memory: Memory::new(count),
        }
    }

    pub fn insert(&mut self, object: O) -> Result<usize, RuntimeError> {
        self.memory.insert(QuasiObject::Local(object))
    }

    pub fn insert_remote(&mut self, share_object: ShareObject<S>) -> Result<usize, RuntimeError> {
        let ShareObject(remote) = share_object;
        self.memory.insert(QuasiObject::Remote(remote))
    }

    pub fn share(&mut self, local_id: usize) -> Result<ShareObject<S>, RuntimeError> {
        let mut queue = VecDeque::new();
        queue.push_back(local_id);
        let mut share_object: Option<ShareObject<S>> = None;
        while let Some(local_id) = queue.pop_front() {
            if let QuasiObject::Remote(_) = self.memory.get(local_id)? {
                continue;
            }
            for holdee_id in self.memory.iter_holdee(local_id) {
                queue.push_back(*holdee_id);
            }
            if let QuasiObject::Local(object) = self.memory.replace(local_id, QuasiObject::Temp)? {
                let remote = Arc::new(RwLock::new(object.into()));
                if share_object.is_none() {
                    share_object = Some(ShareObject(Arc::clone(&remote)));
                }
                self.memory.replace(local_id, QuasiObject::Remote(remote))?;
            } else {
                panic!();
            }
        }
        Ok(share_object.unwrap())
    }

    pub fn read(&self, object_id: usize) -> Result<ReadObject<O, S>, RuntimeError> {
        let read = match self.memory.get(object_id)? {
            QuasiObject::Local(object) => ReadObject::Local(object),
            QuasiObject::Remote(remote) => ReadObject::Remote(
                remote
                    .try_read()
                    .map_err(|_| RuntimeError::AccessConflict)?,
            ),
            QuasiObject::Temp => panic!("inconsistent"),
        };
        Ok(read)
    }

    pub fn write(&mut self, object_id: usize) -> Result<WriteObject<O, S>, RuntimeError> {
        let read = match self.memory.get_mut(object_id)? {
            QuasiObject::Local(object) => WriteObject::Local(object),
            QuasiObject::Remote(remote) => WriteObject::Remote(
                remote
                    .try_write()
                    .map_err(|_| RuntimeError::AccessConflict)?,
            ),
            QuasiObject::Temp => panic!("inconsistent"),
        };
        Ok(read)
    }

    pub fn hold(&mut self, src: usize, dest: usize) -> Result<(), RuntimeError> {
        self.memory.hold(src, dest)
    }

    pub fn unhold(&mut self, src: usize, dest: usize) -> Result<(), RuntimeError> {
        self.memory.unhold(src, dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Object(i64);

    impl Deref for Object {
        type Target = Object;

        fn deref(&self) -> &Self::Target {
            self
        }
    }

    impl DerefMut for Object {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self
        }
    }

    #[test]
    fn test_share_read() {
        let mut run1: Runtime<Object, Object> = Runtime::new(16);
        let mut run2: Runtime<Object, Object> = Runtime::new(16);
        let id1 = run1.insert(Object(42)).unwrap();
        let share = run1.share(id1).unwrap();
        let id2 = run2.insert_remote(share).unwrap();
        assert_eq!(
            &run1.read(id1).unwrap() as &Object,
            &run2.read(id2).unwrap() as &Object
        );
    }

    #[test]
    fn test_share_write() {
        let mut run1: Runtime<Object, Object> = Runtime::new(16);
        let mut run2: Runtime<Object, Object> = Runtime::new(16);
        let id1 = run1.insert(Object(42)).unwrap();
        let share = run1.share(id1).unwrap();
        let id2 = run2.insert_remote(share).unwrap();
        run1.write(id1).unwrap().0 = 43;
        assert_eq!(&run2.read(id2).unwrap() as &Object, &Object(43));
    }

    #[test]
    fn test_access_conflict() {
        let mut run1: Runtime<Object, Object> = Runtime::new(16);
        let mut run2: Runtime<Object, Object> = Runtime::new(16);
        let id1 = run1.insert(Object(42)).unwrap();
        let share = run1.share(id1).unwrap();
        let id2 = run2.insert_remote(share).unwrap();
        let _obj1 = run1.read(id1).unwrap();
        assert!(run2.write(id2).is_err());
    }
}
