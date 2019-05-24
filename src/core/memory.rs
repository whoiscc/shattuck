//

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::core::object::Object;
use crate::core::runtime::Pointer;

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq)]
pub struct Addr(usize);

pub struct Memory {
    max_object_count: usize,
    // the space actually saved (the pointers of) the objects
    objects: HashMap<Addr, Box<dyn Object>>,
    // counter for Addr and PrivAddr
    next_addr: usize,
    // root index in `to_space`
    root_addr: Option<Addr>,
    // reference map, keys and values are indices of `to_space`
    ref_map: HashMap<Addr, HashSet<Addr>>,
}

#[derive(Debug)]
pub enum MemoryError {
    Full,
    InvalidAddr(Addr),
}

impl Display for MemoryError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            MemoryError::Full => write!(f, "memory is full"),
            MemoryError::InvalidAddr(_) => write!(f, "access invalid address"),
        }
    }
}

impl Error for MemoryError {}

impl Memory {
    pub fn new(count: usize) -> Self {
        Memory {
            max_object_count: count,
            objects: HashMap::new(),
            next_addr: 0,
            root_addr: None,
            ref_map: HashMap::new(),
        }
    }

    pub fn append_object(&mut self, object: Box<dyn Object>) -> Result<Addr, MemoryError> {
        if self.objects.len() == self.max_object_count {
            self.collect();
        }

        if self.objects.len() == self.max_object_count {
            return Err(MemoryError::Full);
        }
        let addr = self.allocate_addr();
        self.objects.insert(addr, object);
        self.ref_map.insert(addr, HashSet::new());
        Ok(addr)
    }

    pub fn get_object(&self, addr: Addr) -> Result<&dyn Object, MemoryError> {
        self.objects
            .get(&addr)
            .map(|boxed_obj| &**boxed_obj)
            .ok_or_else(|| MemoryError::InvalidAddr(addr))
    }

    pub fn get_object_mut(&mut self, addr: Addr) -> Result<&mut dyn Object, MemoryError> {
        self.objects
            .get_mut(&addr)
            .map(|boxed_obj| &mut **boxed_obj)
            .ok_or_else(|| MemoryError::InvalidAddr(addr))
    }

    pub fn set_root(&mut self, addr: Addr) -> Result<(), MemoryError> {
        self.get_object(addr)?;
        self.root_addr = Some(addr);
        Ok(())
    }

    pub fn hold(&mut self, holder: Addr, holdee: Addr) -> Result<(), MemoryError> {
        self.get_object(holdee)?;
        self.ref_map
            .get_mut(&holder)
            .ok_or_else(|| MemoryError::InvalidAddr(holder))?
            .insert(holdee);
        Ok(())
    }

    pub fn drop(&mut self, holder: Addr, holdee: Addr) -> Result<(), MemoryError> {
        self.ref_map
            .get_mut(&holder)
            .ok_or_else(|| MemoryError::InvalidAddr(holder))?
            .remove(&holdee);
        Ok(())
    }

    pub fn collect(&mut self) {
        use std::time::Instant;
        let now = Instant::now();

        let mut queue = VecDeque::<Addr>::new();
        let mut dead_set: HashSet<Addr> = self.objects.keys().cloned().collect();
        if let Some(root_addr) = self.root_addr {
            queue.push_back(root_addr);
        }

        // for each alive object
        while let Some(object_addr) = queue.pop_front() {
            // remove it from dead set
            dead_set.remove(&object_addr);
            // queue its holdee
            for holdee_addr in self.ref_map[&object_addr].iter() {
                if dead_set.contains(holdee_addr) {
                    queue.push_back(holdee_addr.to_owned());
                }
            }
        }

        // update objects
        let dead_count = dead_set.len();
        for dead_addr in dead_set {
            self.objects.remove(&dead_addr);
        }

        let alive_count = self.objects.len();
        println!(
            "<shattuck> garbage collected, {} alive, {} dead, duration: {} ms",
            alive_count,
            dead_count,
            now.elapsed().as_micros() as f64 / 1000.0
        );
    }

    fn allocate_addr(&mut self) -> Addr {
        let addr = Addr(self.next_addr);
        self.next_addr += 1;
        addr
    }

    pub fn set_object_property(
        &mut self,
        addr: Addr,
        key: &str,
        new_prop: Addr,
    ) -> Result<(), MemoryError> {
        let object = self.get_object(addr)?;
        if let Some(old_prop) = object.get_property(key) {
            self.drop(addr, old_prop.addr())?;
        }
        let object_mut = self.get_object_mut(addr)?;
        object_mut.set_property(key, Pointer::with_addr(new_prop));
        self.hold(addr, new_prop)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyObject;
    impl Object for DummyObject {
        fn get_property(&self, _key: &str) -> Option<Pointer> {
            panic!()
        }

        fn set_property(&mut self, _key: &str, _new_prop: Pointer) {
            panic!()
        }
    }

    #[test]
    fn store_object_in_memory_and_get_it() {
        let mut mem = Memory::new(16);
        let obj = Box::new(DummyObject);
        let addr = mem.append_object(obj);
        assert!(addr.is_ok());
        let returned_obj = mem.get_object(addr.unwrap());
        assert!(returned_obj.is_ok());
    }

    #[test]
    fn store_fail_when_no_space() {
        let mut mem = Memory::new(1);
        let addr = mem.append_object(Box::new(DummyObject));
        mem.set_root(addr.unwrap()).unwrap();
        assert!(mem.append_object(Box::new(DummyObject)).is_err());
    }

    #[test]
    fn collect_orphan_objects() {
        let mut mem = Memory::new(2);
        let root = mem.append_object(Box::new(DummyObject));
        mem.set_root(root.unwrap()).unwrap();
        let orphan = mem.append_object(Box::new(DummyObject)).unwrap();
        let third = mem.append_object(Box::new(DummyObject));
        assert!(third.is_ok());
        assert!(mem.get_object(orphan).is_err());
        assert!(mem.get_object(third.unwrap()).is_ok());
    }

    #[test]
    fn not_collect_held_objects() {
        let mut mem = Memory::new(2);
        let root = mem.append_object(Box::new(DummyObject)).unwrap();
        mem.set_root(root).unwrap();
        let holdee = mem.append_object(Box::new(DummyObject)).unwrap();
        mem.hold(root, holdee).unwrap();
        let third = mem.append_object(Box::new(DummyObject));
        assert!(third.is_err());
        assert!(mem.get_object(holdee).is_ok());
    }

    use crate::objects::int::IntObject;

    #[test]
    fn same_in_same_out() {
        let mut mem = Memory::new(2);
        let int_obj1 = mem.append_object(Box::new(IntObject(42))).unwrap();
        assert_same_int(&mem, int_obj1, 42);
        let int_obj2 = mem.append_object(Box::new(IntObject(43))).unwrap();
        mem.set_root(int_obj2).unwrap();
        let int_obj3 = mem.append_object(Box::new(IntObject(44))).unwrap();
        assert_same_int(&mem, int_obj2, 43);
        assert_same_int(&mem, int_obj3, 44);
    }

    fn assert_same_int(mem: &Memory, addr: Addr, expect: i64) {
        let returned_obj = mem.get_object(addr).unwrap();
        assert!(returned_obj.as_any().is::<IntObject>());
        let returned_int_obj = returned_obj.as_any().downcast_ref::<IntObject>().unwrap();
        assert_eq!(*returned_int_obj, IntObject(expect));
    }

    #[test]
    fn random_hold() {
        extern crate rand;
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for _ in 0..10 {
            let mut mem = Memory::new(1024);
            let mut addr_vec = Vec::<Addr>::new();
            let mut alive_set = HashSet::<Addr>::new();
            let mut obj_count = 0;
            let root = mem.append_object(Box::new(IntObject(obj_count))).unwrap();
            mem.set_root(root).unwrap();
            obj_count += 1;
            addr_vec.push(root);
            alive_set.insert(root);
            while obj_count < 1024 {
                let obj = mem.append_object(Box::new(IntObject(obj_count))).unwrap();
                obj_count += 1;
                addr_vec.push(obj);
                let mut chance = 0.8;
                while rng.gen::<f64>() < chance {
                    // minus 1 to prevent self-holding
                    let holder = rng.gen_range(0, obj_count - 1) as usize;
                    let holder_addr = addr_vec[holder];
                    mem.hold(holder_addr, obj).unwrap();
                    if alive_set.contains(&holder_addr) {
                        alive_set.insert(obj);
                    }
                    chance -= 0.2;
                }
            }
            let _ = mem.append_object(Box::new(DummyObject)).unwrap();
            for addr in addr_vec {
                if alive_set.contains(&addr) {
                    assert!(mem.get_object(addr).is_ok());
                } else {
                    assert!(mem.get_object(addr).is_err());
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn random_hold_1000() {
        for _ in 0..100 {
            random_hold();
        }
    }
}
