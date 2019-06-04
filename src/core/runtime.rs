//

use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::memory::{AddrGen, Memory, RefMap};
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

pub struct Runtime<L, S, A, G> {
    memory: Memory<QuasiObject<L, S>, A, G>,
    frame_stack: Vec<Frame<A>>,
}

pub struct RuntimeBuilder<L, S, A, G> {
    memory: Memory<QuasiObject<L, S>, A, G>,
}

impl<O, L, S, A, G> RuntimeBuilder<L, S, A, G>
where
    O: ?Sized,
    S: From<L> + DerefMut<Target = O>,
    L: DerefMut<Target = O>,
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn new(count: usize, addr_gen: G) -> Self {
        Self {
            memory: Memory::new(count, addr_gen),
        }
    }

    pub fn insert(&mut self, object: L) -> Result<A, RuntimeError> {
        self.memory.insert(QuasiObject::Local(object))
    }

    pub fn insert_remote(&mut self, share_object: ShareObject<S>) -> Result<A, RuntimeError> {
        let ShareObject(remote) = share_object;
        self.memory.insert(QuasiObject::Remote(remote))
    }
}

impl<O, L, S, A, G> Runtime<L, S, A, G>
where
    O: ?Sized,
    S: From<L> + DerefMut<Target = O>,
    L: DerefMut<Target = O>,
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn new(builder: RuntimeBuilder<L, S, A, G>, context: &A) -> Self {
        Self {
            memory: builder.memory,
            frame_stack: vec![Frame::new(context)],
        }
    }

    pub fn context(&self) -> &A {
        &self.frame_stack.last().expect("current frame").context
    }

    pub fn insert(&mut self, object: L) -> Result<A, RuntimeError> {
        self.memory.insert(QuasiObject::Local(object))
    }

    pub fn insert_remote(&mut self, share_object: ShareObject<S>) -> Result<A, RuntimeError> {
        let ShareObject(remote) = share_object;
        self.memory.insert(QuasiObject::Remote(remote))
    }

    pub fn share(&mut self, local_id: &A) -> Result<ShareObject<S>, RuntimeError> {
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

    pub fn read(&self, object_id: &A) -> Result<ReadObject<L, S>, RuntimeError> {
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

    pub fn write(&mut self, object_id: &A) -> Result<WriteObject<L, S>, RuntimeError> {
        let write = match self.memory.get_mut(object_id)? {
            QuasiObject::Local(object) => WriteObject::Local(object),
            QuasiObject::Remote(remote) => WriteObject::Remote(
                remote
                    .try_write()
                    .map_err(|_| RuntimeError::AccessConflict)?,
            ),
            QuasiObject::Temp => panic!("inconsistent"),
        };
        Ok(write)
    }

    pub fn ref_map(&mut self) -> &mut RefMap<A> {
        &mut self.memory.ref_map
    }

    pub fn addr_gen(&self) -> &G {
        &self.memory.next_addr
    }
}

pub trait Method<L, S, A, G> {
    fn run(&self, runtime: &mut Runtime<L, S, A, G>) -> Result<(), RuntimeError>;
}

pub trait AsMethod<L, S, A, G> {
    fn as_method(&self) -> Result<&dyn Method<L, S, A, G>, RuntimeError> {
        Err(RuntimeError::NotCallable)
    }
}

impl<O, L, S, A, G> Runtime<L, S, A, G>
where
    O: ?Sized,
    S: From<L> + DerefMut<Target = O> + AsMethod<L, S, A, G>,
    L: DerefMut<Target = O>,
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn call(&mut self, method: &A) -> Result<(), RuntimeError> {
        let share_object = Arc::clone(&self.share(method)?.0);
        let read_method = share_object
            .read()
            .map_err(|_| RuntimeError::AccessConflict)?;
        self.frame_stack.push(Frame::new(method));
        read_method.as_method()?.run(self)?;
        self.frame_stack.pop().expect("pop previously pushed frame");
        Ok(())
    }
}

struct Frame<A> {
    context: A,
}

impl<A> Frame<A>
where
    A: Clone,
{
    fn new(context: &A) -> Self {
        Self {
            context: context.to_owned(),
        }
    }
}
