//

use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::VecDeque;
use std::mem;
use std::ops::{Deref, DerefMut};

use crate::core::error::{Error, Result};
use crate::core::object::{Object, SyncMut, SyncObject, SyncRef, ToSync};

enum Dual {
    Local(RefCell<Object>),
    Shared(SyncObject),
}

pub enum DualRef<'a> {
    Local(Ref<'a, Object>),
    Shared(SyncRef<'a>),
}

pub enum DualMut<'a> {
    Local(RefMut<'a, Object>),
    Shared(SyncMut<'a>),
}

impl Dual {
    fn get_ref(&self) -> Result<DualRef> {
        Ok(match self {
            Dual::Local(object) => {
                DualRef::Local(object.try_borrow().map_err(|_| Error::ViolateSync)?)
            }
            Dual::Shared(object) => DualRef::Shared(object.get_ref()?),
        })
    }

    fn get_mut(&self) -> Result<DualMut> {
        Ok(match self {
            Dual::Local(object) => {
                DualMut::Local(object.try_borrow_mut().map_err(|_| Error::ViolateSync)?)
            }
            Dual::Shared(object) => DualMut::Shared(object.get_mut()?),
        })
    }

    fn into_shared<T: Any + ToSync>(self) -> Result<Self> {
        let object = match self {
            Dual::Local(object) => object.into_inner().into_sync::<T>()?,
            Dual::Shared(object) => object,
        };
        Ok(Dual::Shared(object))
    }

    fn get_holdee(&self) -> Vec<Address> {
        match self {
            // it is safe to `borrow` here
            // this method may be called by `Memory::collect` only
            // which keeps a mutable reference to the whole `Memory`
            // thus this `Dual` owned by it cannot be referenced at the same time
            Dual::Local(object) => object.borrow().get_holdee(),
            Dual::Shared(object) => object.get_holdee(),
        }
    }
}

impl<'a> DualRef<'a> {
    pub fn as_ref<T, U, I>(&self) -> Result<&I>
    where
        T: Any + Deref<Target = I>,
        U: Any + Deref<Target = I>,
        I: Any,
    {
        match self {
            DualRef::Local(object) => object.as_ref(),
            DualRef::Shared(object) => object.as_ref(),
        }
    }
}

impl<'a> DualMut<'a> {
    pub fn as_ref<T, U, I>(&self) -> Result<&I>
    where
        T: Any + Deref<Target = I>,
        U: Any + Deref<Target = I>,
        I: Any,
    {
        match self {
            DualMut::Local(object) => object.as_ref(),
            DualMut::Shared(object) => object.as_ref(),
        }
    }

    pub fn as_mut<T, U, I>(&mut self) -> Result<&mut I>
    where
        T: Any + DerefMut<Target = I>,
        U: Any + DerefMut<Target = I>,
        I: Any,
    {
        match self {
            DualMut::Local(object) => object.as_mut(),
            DualMut::Shared(object) => object.as_mut(),
        }
    }
}

struct Slot {
    dual: Dual,
    mark: bool,
}

pub struct Memory {
    slots: Vec<Address>,
    n_slots_max: usize,
    entry: Option<Address>,
}

#[derive(Clone, Copy)]
pub struct Address(*mut Slot);

impl Address {
    fn new(dual: Dual) -> Self {
        Self(Box::leak(Box::new(Slot { dual, mark: false })))
    }

    fn slot_ref(&self) -> &Slot {
        unsafe { self.0.as_ref().unwrap() }
    }

    fn slot_mut(&mut self) -> &mut Slot {
        unsafe { self.0.as_mut().unwrap() }
    }

    pub fn get_ref(&self) -> Result<DualRef> {
        self.slot_ref().dual.get_ref()
    }

    pub fn get_mut(&mut self) -> Result<DualMut> {
        self.slot_mut().dual.get_mut()
    }

    fn get_holdee(&self) -> Vec<Address> {
        self.slot_ref().dual.get_holdee()
    }

    fn mark(&mut self) {
        self.slot_mut().mark = true;
    }

    fn unmark(&mut self) {
        self.slot_mut().mark = false;
    }

    fn is_marked(&self) -> bool {
        self.slot_ref().mark
    }
}

impl Memory {
    pub fn new(n_slots_max: usize) -> Self {
        Self {
            n_slots_max,
            slots: Vec::new(),
            entry: None,
        }
    }

    fn insert_dual(&mut self, dual: Dual) -> Result<Address> {
        if self.n_object() == self.n_slots_max {
            self.collect();
        }
        if self.n_object() == self.n_slots_max {
            return Err(Error::OutOfMemory);
        }
        let addr = Address::new(dual);
        self.slots.push(addr);
        Ok(addr)
    }

    pub fn insert_local(&mut self, object: Object) -> Result<Address> {
        self.insert_dual(Dual::Local(RefCell::new(object)))
    }

    pub fn insert_shared(&mut self, object: SyncObject) -> Result<Address> {
        self.insert_dual(Dual::Shared(object))
    }
}

impl Address {
    fn release(self) {
        let slot = unsafe { Box::from_raw(self.0) };
        mem::drop(slot);
    }

    pub fn share<T: Any + ToSync>(&mut self) -> Result<SyncObject> {
        let slot = unsafe { *Box::from_raw(self.0) };
        let shared = slot.dual.into_shared::<T>()?;
        self.0 = Box::leak(Box::new(Slot {
            dual: shared,
            mark: slot.mark,
        }));
        if let Dual::Shared(sync_object) = &self.slot_ref().dual {
            Ok(sync_object.clone())
        } else {
            unreachable!()
        }
    }
}

impl Memory {
    pub fn set_entry(&mut self, entry: Address) {
        self.entry = Some(entry);
    }

    pub fn collect(&mut self) {
        let mut que = VecDeque::new();
        if let Some(entry) = self.entry {
            que.push_back(entry);
        }
        while let Some(mut addr) = que.pop_front() {
            addr.mark();
            for holdee in addr.get_holdee() {
                if !holdee.is_marked() {
                    que.push_back(holdee);
                }
            }
        }
        // ugly here
        self.slots.retain(|addr| {
            let marked = addr.is_marked();
            addr.to_owned().unmark();
            if !marked {
                addr.release();
            }
            marked
        });
    }

    pub fn n_object(&self) -> usize {
        self.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::object::{GetHoldeeOfObject, GetHoldeeOfSyncObject};

    struct Int(i32);

    unsafe impl GetHoldeeOfObject for Int {
        fn get_holdee(_object: &Object) -> Vec<Address> {
            Vec::new()
        }
    }

    unsafe impl GetHoldeeOfSyncObject for Int {
        fn get_holdee(_object: &SyncObject) -> Vec<Address> {
            Vec::new()
        }
    }

    #[test]
    fn memory_insert() {
        let mut mem = Memory::new(16);
        let mut addr = mem.insert_local(Object::new(Int(42))).unwrap();
        assert_eq!(
            addr.get_ref().unwrap().as_ref::<&Int, &Int, _>().unwrap().0,
            42
        );
        *addr
            .get_mut()
            .unwrap()
            .as_mut::<&mut Int, &mut Int, _>()
            .unwrap() = Int(43);
        assert_eq!(
            addr.get_ref().unwrap().as_ref::<&Int, &Int, _>().unwrap().0,
            43
        );
    }

    impl ToSync for Int {
        type Target = Int;

        fn to_sync(self) -> Result<Self::Target> {
            Ok(self)
        }
    }

    #[test]
    fn make_shared() {
        let mut mem = Memory::new(16);
        let mut addr = mem.insert_local(Object::new(Int(42))).unwrap();
        assert_eq!(
            addr.get_ref().unwrap().as_ref::<&Int, &Int, _>().unwrap().0,
            42
        );
        addr.share::<Int>().unwrap();
        assert_eq!(
            addr.get_ref().unwrap().as_ref::<&Int, &Int, _>().unwrap().0,
            42
        );
    }

    #[test]
    fn simple_collect() {
        let mut mem = Memory::new(16);
        let _addr = mem.insert_local(Object::new(Int(42))).unwrap();
        assert_eq!(mem.n_object(), 1);
        mem.collect();
        assert_eq!(mem.n_object(), 0);
    }

    struct Node(Vec<Address>);

    unsafe impl GetHoldeeOfObject for Node {
        fn get_holdee(object: &Object) -> Vec<Address> {
            object.as_ref::<Node>().unwrap().0.to_owned()
        }
    }

    unsafe impl GetHoldeeOfSyncObject for Node {
        fn get_holdee(object: &SyncObject) -> Vec<Address> {
            object
                .get_ref()
                .unwrap()
                .as_ref::<Node>()
                .unwrap()
                .0
                .to_owned()
        }
    }

    #[test]
    fn keep_alive_after_collect() {
        let mut mem = Memory::new(16);
        let holdee = mem.insert_local(Object::new(Node(Vec::new()))).unwrap();
        let mut holder = mem.insert_local(Object::new(Node(Vec::new()))).unwrap();
        holder
            .get_mut()
            .unwrap()
            .as_mut::<&mut Node, &mut Node, _>()
            .unwrap()
            .0
            .push(holdee);
        mem.set_entry(holder);
        mem.collect();
        assert_eq!(mem.n_object(), 2);
        mem.collect();
        assert_eq!(mem.n_object(), 2);
        mem.collect();
        assert_eq!(mem.n_object(), 2);
        mem.set_entry(holdee);
        mem.collect();
        assert_eq!(mem.n_object(), 1);
    }

    use std::thread;

    #[test]
    fn simple_share() {
        let mut mem = Memory::new(16);
        let mut addr = mem.insert_local(Object::new(Int(42))).unwrap();
        let shared = addr.share::<Int>().unwrap();
        let handle = thread::spawn(move || {
            let mut mem = Memory::new(16);
            let mut addr = mem.insert_shared(shared).unwrap();
            assert_eq!(
                addr.get_ref().unwrap().as_ref::<&Int, &Int, _>().unwrap().0,
                42
            );
            *addr
                .get_mut()
                .unwrap()
                .as_mut::<&mut Int, &mut Int, _>()
                .unwrap() = Int(43);
        });
        handle.join().unwrap();
        assert_eq!(
            addr.get_ref().unwrap().as_ref::<&Int, &Int, _>().unwrap().0,
            43
        );
    }
}
