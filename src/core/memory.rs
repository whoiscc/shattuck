//

use std::any::Any;
use std::collections::VecDeque;
use std::mem;
use std::ptr;

use crate::core::error::{Error, Result};
use crate::core::object::{Object, SyncMut, SyncObject, SyncRef};

enum Dual {
    Local(Object),
    Shared(SyncObject),
}

pub enum DualRef<'a> {
    Local(&'a Object),
    Shared(SyncRef<'a>),
}

pub enum DualMut<'a> {
    Local(&'a mut Object),
    Shared(SyncMut<'a>),
}

impl Dual {
    fn get_ref(&self) -> Result<DualRef> {
        Ok(match self {
            Dual::Local(object) => DualRef::Local(object),
            Dual::Shared(object) => DualRef::Shared(object.get_ref()?),
        })
    }

    fn get_mut(&mut self) -> Result<DualMut> {
        Ok(match self {
            Dual::Local(object) => DualMut::Local(object),
            // `SyncObject::get_mut` doesn't require `&mut self`
            // but requiring it match semantic better
            Dual::Shared(object) => DualMut::Shared(object.get_mut()?),
        })
    }

    fn sync_ref(&self) -> DualRef {
        match self {
            Dual::Local(object) => DualRef::Local(object),
            Dual::Shared(object) => DualRef::Shared(object.sync_ref()),
        }
    }

    fn sync_mut(&mut self) -> DualMut {
        match self {
            Dual::Local(object) => DualMut::Local(object),
            Dual::Shared(object) => DualMut::Shared(object.sync_mut()),
        }
    }

    fn into_shared(self) -> Result<Self> {
        let object = match self {
            Dual::Local(object) => object.into_sync()?,
            Dual::Shared(object) => object,
        };
        Ok(Dual::Shared(object))
    }

    fn get_holdee(&self) -> Vec<Address> {
        match self {
            Dual::Local(object) => object.get_holdee(),
            Dual::Shared(_) => Vec::new(),
        }
    }
}

impl<'a> DualRef<'a> {
    pub fn as_local_ref<T: Any>(&self) -> Result<&T> {
        if let DualRef::Local(object) = self {
            object.as_ref()
        } else {
            Err(Error::ExpectLocal)
        }
    }

    pub fn as_shared_ref<T: Any>(&self) -> Result<&T> {
        if let DualRef::Shared(object) = self {
            object.as_ref()
        } else {
            Err(Error::ExpectShared)
        }
    }

    pub fn as_dual_ref<L, S, LF, SF, R>(&self, local_fn: LF, shared_fn: SF) -> Result<R>
    where
        L: Any,
        S: Any,
        LF: FnOnce(&L) -> Result<R>,
        SF: FnOnce(&S) -> Result<R>,
    {
        match self {
            DualRef::Local(object) => local_fn(object.as_ref::<L>()?),
            DualRef::Shared(object) => shared_fn(object.as_ref::<S>()?),
        }
    }

    pub fn as_ref<T: Any>(&self) -> Result<&T> {
        match self {
            DualRef::Local(object) => object.as_ref(),
            DualRef::Shared(object) => object.as_ref(),
        }
    }
}

impl<'a> DualMut<'a> {
    pub fn as_local_ref<T: Any>(&self) -> Result<&T> {
        if let DualMut::Local(object) = self {
            object.as_ref()
        } else {
            Err(Error::ExpectLocal)
        }
    }

    pub fn as_shared_ref<T: Any>(&self) -> Result<&T> {
        if let DualMut::Shared(object) = self {
            object.as_ref()
        } else {
            Err(Error::ExpectShared)
        }
    }

    pub fn as_dual_ref<L, S, LF, SF, R>(&self, local_fn: LF, shared_fn: SF) -> Result<R>
    where
        L: Any,
        S: Any,
        LF: FnOnce(&L) -> Result<R>,
        SF: FnOnce(&S) -> Result<R>,
    {
        match self {
            DualMut::Local(object) => local_fn(object.as_ref::<L>()?),
            DualMut::Shared(object) => shared_fn(object.as_ref::<S>()?),
        }
    }

    pub fn as_ref<T: Any>(&self) -> Result<&T> {
        match self {
            DualMut::Local(object) => object.as_ref(),
            DualMut::Shared(object) => object.as_ref(),
        }
    }

    pub fn as_local_mut<T: Any>(&mut self) -> Result<&mut T> {
        if let DualMut::Local(object) = self {
            object.as_mut()
        } else {
            Err(Error::ExpectLocal)
        }
    }

    pub fn as_shared_mut<T: Any>(&mut self) -> Result<&mut T> {
        if let DualMut::Shared(object) = self {
            object.as_mut()
        } else {
            Err(Error::ExpectShared)
        }
    }

    pub fn as_dual_mut<L, S, LF, SF, R>(&mut self, local_fn: LF, shared_fn: SF) -> Result<R>
    where
        L: Any,
        S: Any,
        LF: FnOnce(&mut L) -> Result<R>,
        SF: FnOnce(&mut S) -> Result<R>,
    {
        match self {
            DualMut::Local(object) => local_fn(object.as_mut::<L>()?),
            DualMut::Shared(object) => shared_fn(object.as_mut::<S>()?),
        }
    }

    pub fn as_mut<T: Any>(&mut self) -> Result<&mut T> {
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address(*mut Slot);

impl Address {
    fn new(dual: Dual) -> Self {
        Self(Box::into_raw(Box::new(Slot { dual, mark: false })))
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

    pub fn sync_ref(&self) -> DualRef {
        self.slot_ref().dual.sync_ref()
    }

    pub fn sync_mut(&mut self) -> DualMut {
        self.slot_mut().dual.sync_mut()
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
        self.insert_dual(Dual::Local(object))
    }

    pub fn insert_shared(&mut self, object: SyncObject) -> Result<Address> {
        self.insert_dual(Dual::Shared(object))
    }
}

impl Address {
    fn release(mut self) {
        let slot = unsafe { Box::from_raw(self.0) };
        mem::drop(slot);
        self.0 = unsafe { mem::zeroed() };
    }

    pub fn share(&mut self) -> Result<SyncObject> {
        unsafe {
            let mut slot = ptr::read(self.0);
            slot.dual = slot.dual.into_shared()?;
            ptr::write(self.0, slot);
        }
        if let Dual::Shared(sync_object) = &self.slot_ref().dual {
            Ok(sync_object.clone())
        } else {
            unreachable!()
        }
    }
}

impl Memory {
    pub(crate) fn set_entry(&mut self, entry: Address) {
        self.entry = Some(entry);
    }

    pub fn collect(&mut self) {
        use std::thread;
        use std::time::Instant;
        let start = Instant::now();

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

        let before_collect = self.slots.len();
        // ugly here
        self.slots.retain(|addr| {
            let marked = addr.is_marked();
            if !marked {
                addr.release();
            } else {
                addr.to_owned().unmark();
            }
            marked
        });

        println!(
            "<shattuck> {:?} collected {} objects in {} us.",
            thread::current().id(),
            before_collect - self.slots.len(),
            start.elapsed().as_micros(),
        );
    }

    pub fn n_object(&self) -> usize {
        self.slots.len()
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        for addr in self.slots.iter() {
            addr.release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::object::{GetHoldee, ToSync};

    struct Int(i32);

    unsafe impl GetHoldee for Int {
        fn get_holdee(&self) -> Vec<Address> {
            Vec::new()
        }
    }

    #[test]
    fn memory_insert() {
        let mut mem = Memory::new(16);
        let mut addr = mem.insert_local(Object::new(Int(42))).unwrap();
        assert_eq!(addr.get_ref().unwrap().as_ref::<Int>().unwrap().0, 42);
        *addr.get_mut().unwrap().as_mut::<Int>().unwrap() = Int(43);
        assert_eq!(addr.get_ref().unwrap().as_ref::<Int>().unwrap().0, 43);
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
        assert_eq!(addr.get_ref().unwrap().as_ref::<Int>().unwrap().0, 42);
        addr.share().unwrap();
        assert_eq!(addr.get_ref().unwrap().as_ref::<Int>().unwrap().0, 42);
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

    unsafe impl GetHoldee for Node {
        fn get_holdee(&self) -> Vec<Address> {
            self.0.to_owned()
        }
    }

    use crate::core::object::NoSync;

    impl NoSync for Node {}

    #[test]
    fn keep_alive_after_collect() {
        let mut mem = Memory::new(16);
        let holdee = mem.insert_local(Object::new(Node(Vec::new()))).unwrap();
        let mut holder = mem.insert_local(Object::new(Node(Vec::new()))).unwrap();
        holder
            .get_mut()
            .unwrap()
            .as_mut::<Node>()
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
        let shared = addr.share().unwrap();
        let handle = thread::spawn(move || {
            let mut mem = Memory::new(16);
            let mut addr = mem.insert_shared(shared).unwrap();
            assert_eq!(addr.get_ref().unwrap().as_ref::<Int>().unwrap().0, 42);
            *addr.get_mut().unwrap().as_mut::<Int>().unwrap() = Int(43);
        });
        handle.join().unwrap();
        assert_eq!(addr.get_ref().unwrap().as_ref::<Int>().unwrap().0, 43);
    }

    use std::collections::HashSet;

    extern crate rand;
    use rand::{thread_rng, Rng};

    #[test]
    fn random_hold() {
        let total = 4096;
        let mut mem = Memory::new(total);
        let mut alive_set = HashSet::new();
        let mut addr_list = Vec::new();
        let entry = mem.insert_local(Object::new(Node(Vec::new()))).unwrap();
        mem.set_entry(entry);
        alive_set.insert(0);
        addr_list.push(entry);

        let mut rng = thread_rng();
        for i in 1..total {
            let addr = mem.insert_local(Object::new(Node(Vec::new()))).unwrap();
            addr_list.push(addr);
            let mut chance = 0.8;
            while rng.gen::<f64>() < chance {
                let holder = rng.gen_range(0, i) as usize;
                addr_list[holder]
                    .get_mut()
                    .unwrap()
                    .as_mut::<Node>()
                    .unwrap()
                    .0
                    .push(addr);
                if alive_set.contains(&holder) {
                    alive_set.insert(i);
                }
                chance -= 0.2;
            }
        }
        assert_eq!(mem.n_object(), total);
        mem.collect();
        assert_eq!(mem.n_object(), alive_set.len());
    }

    #[test]
    #[ignore]
    fn random_hold_1000() {
        for _ in 0..1000 {
            random_hold();
        }
    }
}
