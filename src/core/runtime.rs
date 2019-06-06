//

use std::collections::VecDeque;
use std::hash::Hash;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::memory::{AddrGen, Memory, RefMap};
use crate::core::runtime_error::RuntimeError;

pub(crate) enum QuasiObject<L, S> {
    Local(L),
    Shared(Arc<RwLock<S>>),
    Trans,
}

pub enum ReadObject<'a, L, S> {
    Local(&'a L),
    Shared(RwLockReadGuard<'a, S>),
}

impl<'a, O, L, S> Deref for ReadObject<'a, L, S>
where
    O: ?Sized,
    L: Deref<Target = O>,
    S: Deref<Target = O>,
{
    type Target = O;
    fn deref(&self) -> &Self::Target {
        match self {
            ReadObject::Local(local) => local as &Self::Target,
            ReadObject::Shared(shared) => shared as &Self::Target,
        }
    }
}

pub enum WriteObject<'a, L, S> {
    Local(&'a mut L),
    Shared(RwLockWriteGuard<'a, S>),
}

impl<'a, O, L, S> Deref for WriteObject<'a, L, S>
where
    O: ?Sized,
    L: Deref<Target = O>,
    S: Deref<Target = O>,
{
    type Target = O;
    fn deref(&self) -> &Self::Target {
        match self {
            WriteObject::Local(local) => local as &Self::Target,
            WriteObject::Shared(shared) => shared as &Self::Target,
        }
    }
}

impl<'a, O, L, S> DerefMut for WriteObject<'a, L, S>
where
    O: ?Sized,
    L: DerefMut<Target = O>,
    S: DerefMut<Target = O>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            WriteObject::Local(local) => local as &mut Self::Target,
            WriteObject::Shared(shared) => shared as &mut Self::Target,
        }
    }
}

pub struct ShareObject<S>(Arc<RwLock<S>>);

pub trait IntoShared<S, I> {
    fn into_shared(self) -> Result<(S, I), RuntimeError>;
}

pub struct Runtime<L, S, A, G, I> {
    memory: Memory<QuasiObject<L, S>, A, G>,
    frame_stack: Vec<Frame<A>>,
    phantom: PhantomData<I>,
}

impl<O, L, S, A, G, I> Runtime<L, S, A, G, I>
where
    O: ?Sized,
    L: DerefMut<Target = O> + IntoShared<S, I>,
    S: DerefMut<Target = O>,
    I: Iterator<Item = A>,
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn share(&mut self, addr: &A) -> Result<ShareObject<S>, RuntimeError> {
        let mut q = VecDeque::new();
        q.push_back(addr.clone());
        let mut share_object = None;

        while let Some(addr) = q.pop_front() {
            let shared = match self.memory.replace(&addr, QuasiObject::Trans)? {
                QuasiObject::Local(local) => {
                    let (shared, children) = local.into_shared()?;
                    for child in children {
                        q.push_back(child);
                    }
                    Arc::new(RwLock::new(shared))
                }
                QuasiObject::Trans => panic!("inconsistent"),
                QuasiObject::Shared(shared) => shared,
            };
            self.memory
                .replace(&addr, QuasiObject::Shared(Arc::clone(&shared)))?;
            if share_object.is_none() {
                share_object = Some(ShareObject(Arc::clone(&shared)));
            }
        }
        Ok(share_object.unwrap())
    }

    pub fn read(&self, addr: &A) -> Result<ReadObject<'_, L, S>, RuntimeError> {
        Ok(match self.memory.get(addr)? {
            QuasiObject::Local(local) => ReadObject::Local(local),
            QuasiObject::Shared(shared) => ReadObject::Shared(
                shared
                    .try_read()
                    .map_err(|_| RuntimeError::AccessConflict)?,
            ),
            QuasiObject::Trans => panic!("inconsistent"),
        })
    }

    pub fn write(&mut self, addr: &A) -> Result<WriteObject<'_, L, S>, RuntimeError> {
        Ok(match self.memory.get_mut(addr)? {
            QuasiObject::Local(local) => WriteObject::Local(local),
            QuasiObject::Shared(shared) => WriteObject::Shared(
                shared
                    .try_write()
                    .map_err(|_| RuntimeError::AccessConflict)?,
            ),
            QuasiObject::Trans => panic!("inconsistent"),
        })
    }
}

impl<L, S, A, G, I> Runtime<L, S, A, G, I>
where
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn insert_local(&mut self, local: L) -> Result<A, RuntimeError> {
        self.memory.insert(QuasiObject::Local(local))
    }

    pub fn insert_shared(&mut self, shared: ShareObject<S>) -> Result<A, RuntimeError> {
        self.memory.insert(QuasiObject::Shared(shared.0))
    }

    pub fn ref_map(&mut self) -> &mut RefMap<A> {
        &mut self.memory.ref_map
    }
}

struct Frame<A> {
    context: A,
}

pub struct Builder<L, S, A, G, I> {
    runtime: Runtime<L, S, A, G, I>,
}

impl<L, S, A, G, I> Builder<L, S, A, G, I>
where
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn new(count: usize, gen: G) -> Self {
        Builder {
            runtime: Runtime {
                memory: Memory::new(count, gen),
                frame_stack: Vec::new(),
                phantom: PhantomData,
            },
        }
    }

    pub fn insert_local(&mut self, local: L) -> Result<A, RuntimeError> {
        self.runtime.insert_local(local)
    }

    pub fn insert_shared(&mut self, shared: ShareObject<S>) -> Result<A, RuntimeError> {
        self.runtime.insert_shared(shared)
    }
}

impl<L, S, A, G, I> Runtime<L, S, A, G, I> {
    pub fn new(builder: Builder<L, S, A, G, I>, context: A) -> Self {
        Self {
            memory: builder.runtime.memory,
            frame_stack: vec![Frame { context }],
            phantom: PhantomData,
        }
    }

    pub fn context(&self) -> &A {
        &self.frame_stack.last().unwrap().context
    }
}
