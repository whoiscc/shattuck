//

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::memory::Memory;
use crate::core::runtime_error::RuntimeError;

enum QuasiObject<L, S> {
    Local(L),
    Remote(Arc<RwLock<S>>),
    Temp,
}

pub enum ReadObject<'a, L, S> {
    Local(&'a L),
    Remote(RwLockReadGuard<'a, S>),
}

impl<'a, O, L, S> Deref for ReadObject<'a, L, S>
where
    O: ?Sized,
    S: Deref<Target = O>,
    L: Deref<Target = O>,
{
    type Target = O;

    fn deref(&self) -> &Self::Target {
        match self {
            ReadObject::Local(object) => object as &Self::Target,
            ReadObject::Remote(guard) => guard as &Self::Target,
        }
    }
}

pub enum WriteObject<'a, L, S> {
    Local(&'a mut L),
    Remote(RwLockWriteGuard<'a, S>),
}

impl<'a, O, L, S> Deref for WriteObject<'a, L, S>
where
    O: ?Sized,
    S: Deref<Target = O>,
    L: Deref<Target = O>,
{
    type Target = O;

    fn deref(&self) -> &Self::Target {
        match self {
            WriteObject::Local(object) => object as &Self::Target,
            WriteObject::Remote(guard) => guard as &Self::Target,
        }
    }
}

impl<'a, O, L, S> DerefMut for WriteObject<'a, L, S>
where
    O: ?Sized,
    S: DerefMut<Target = O>,
    L: DerefMut<Target = O>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            WriteObject::Local(object) => object as &mut Self::Target,
            WriteObject::Remote(guard) => guard as &mut Self::Target,
        }
    }
}

pub struct ShareObject<S>(Arc<RwLock<S>>);

pub struct Runtime<L, S> {
    memory: Memory<QuasiObject<L, S>>,
}

impl<O, L, S> Runtime<L, S>
where
    O: ?Sized,
    S: From<L> + DerefMut<Target = O>,
    L: DerefMut<Target = O>,
{
    pub fn new(count: usize) -> Self {
        Self {
            memory: Memory::new(count),
        }
    }

    pub fn insert(&mut self, object: L) -> Result<usize, RuntimeError> {
        self.memory.insert(QuasiObject::Local(object))
    }

    pub fn insert_remote(&mut self, share_object: ShareObject<S>) -> Result<usize, RuntimeError> {
        let ShareObject(remote) = share_object;
        self.memory.insert(QuasiObject::Remote(remote))
    }

    pub fn share(&mut self, local_id: usize) -> Result<ShareObject<S>, RuntimeError> {
        let remote = if let QuasiObject::Remote(remote) = self.memory.get(local_id)? {
            Arc::clone(remote)
        } else if let QuasiObject::Local(object) =
            self.memory.replace(local_id, QuasiObject::Temp)?
        {
            let remote = Arc::new(RwLock::new(object.into()));
            self.memory
                .replace(local_id, QuasiObject::Remote(Arc::clone(&remote)))?;
            remote
        } else {
            panic!();
        };
        Ok(ShareObject(remote))
    }

    pub fn read(&self, object_id: usize) -> Result<ReadObject<L, S>, RuntimeError> {
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

    pub fn write(&mut self, object_id: usize) -> Result<WriteObject<L, S>, RuntimeError> {
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

    type SimpleRuntime<T> = Runtime<T, T>;

    #[test]
    fn test_share_read() {
        let mut run1 = SimpleRuntime::new(16);
        let mut run2 = SimpleRuntime::new(16);
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
        let mut run1 = SimpleRuntime::new(16);
        let mut run2 = SimpleRuntime::new(16);
        let id1 = run1.insert(Object(42)).unwrap();
        let share = run1.share(id1).unwrap();
        let id2 = run2.insert_remote(share).unwrap();
        run1.write(id1).unwrap().0 = 43;
        assert_eq!(&run2.read(id2).unwrap() as &Object, &Object(43));
    }

    #[test]
    fn test_access_conflict() {
        let mut run1 = SimpleRuntime::new(16);
        let mut run2 = SimpleRuntime::new(16);
        let id1 = run1.insert(Object(42)).unwrap();
        let share = run1.share(id1).unwrap();
        let id2 = run2.insert_remote(share).unwrap();
        let _obj1 = run1.read(id1).unwrap();
        assert!(run2.write(id2).is_err());
    }
}
