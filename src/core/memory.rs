//

use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::interp::Name;
use crate::core::object::Object;

#[derive(Clone, Copy, Hash, Debug)]
pub struct Addr(usize);

impl PartialEq for Addr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Addr {}

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

impl Memory {
    pub fn with_max_object_count(count: usize) -> Self {
        Memory {
            max_object_count: count,
            objects: HashMap::new(),
            next_addr: 0,
            root_addr: None,
            ref_map: HashMap::new(),
        }
    }

    pub fn append_object(&mut self, object: Box<dyn Object>) -> Option<Addr> {
        if self.objects.len() == self.max_object_count {
            self.collect();
        }

        if self.objects.len() == self.max_object_count {
            return None;
        }
        let addr = self.allocate_addr();
        self.objects.insert(addr, object);
        self.ref_map.insert(addr, HashSet::new());
        Some(addr)
    }

    pub fn get_object(&self, addr: Addr) -> Option<&Box<dyn Object>> {
        self.objects.get(&addr)
    }

    pub fn get_object_mut(&mut self, addr: Addr) -> Option<&mut Box<dyn Object>> {
        self.objects.get_mut(&addr)
    }

    pub fn set_root(&mut self, addr: Addr) {
        assert!(self.get_object(addr).is_some());
        self.root_addr = Some(addr);
    }

    pub fn hold(&mut self, holder: Addr, holdee: Addr) {
        self.ref_map.get_mut(&holder).unwrap().insert(holdee);
    }

    pub fn drop(&mut self, holder: Addr, holdee: Addr) {
        self.ref_map.get_mut(&holder).unwrap().remove(&holdee);
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

    pub fn set_object_property(&mut self, addr: Addr, key: &str, new_prop: Addr) {
        let object = self.get_object(addr).unwrap();
        if let Some(old_prop) = object.get_property(key) {
            self.drop(addr, old_prop.addr());
        }
        let object_mut = self.get_object_mut(addr).unwrap();
        object_mut.set_property(key, Name::with_addr(new_prop));
        self.hold(addr, new_prop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyObject;
    impl Object for DummyObject {
        fn get_property(&self, _key: &str) -> Option<Name> {
            panic!()
        }

        fn set_property(&mut self, _key: &str, _new_prop: Name) {
            panic!()
        }
    }

    #[test]
    fn store_object_in_memory_and_get_it() {
        let mut mem = Memory::with_max_object_count(16);
        let obj = Box::new(DummyObject);
        let addr = mem.append_object(obj);
        assert!(addr.is_some());
        let returned_obj = mem.get_object(addr.unwrap());
        assert!(returned_obj.is_some());
    }

    #[test]
    fn store_fail_when_no_space() {
        let mut mem = Memory::with_max_object_count(1);
        let addr = mem.append_object(Box::new(DummyObject));
        mem.set_root(addr.unwrap());
        assert!(mem.append_object(Box::new(DummyObject)).is_none());
    }

    #[test]
    fn collect_orphan_objects() {
        let mut mem = Memory::with_max_object_count(2);
        let root = mem.append_object(Box::new(DummyObject));
        mem.set_root(root.unwrap());
        let orphan = mem.append_object(Box::new(DummyObject)).unwrap();
        let third = mem.append_object(Box::new(DummyObject));
        assert!(third.is_some());
        assert!(mem.get_object(orphan).is_none());
        assert!(mem.get_object(third.unwrap()).is_some());
    }

    #[test]
    fn not_collect_held_objects() {
        let mut mem = Memory::with_max_object_count(2);
        let root = mem.append_object(Box::new(DummyObject)).unwrap();
        mem.set_root(root);
        let holdee = mem.append_object(Box::new(DummyObject)).unwrap();
        mem.hold(root, holdee);
        let third = mem.append_object(Box::new(DummyObject));
        assert!(third.is_none());
        assert!(mem.get_object(holdee).is_some());
    }

    use crate::core::object::{as_type, check_type};
    use crate::objects::IntObject;

    #[test]
    fn same_in_same_out() {
        let mut mem = Memory::with_max_object_count(2);
        let int_obj1 = mem.append_object(Box::new(IntObject(42))).unwrap();
        assert_same_int(&mem, int_obj1, 42);
        let int_obj2 = mem.append_object(Box::new(IntObject(43))).unwrap();
        mem.set_root(int_obj2);
        let int_obj3 = mem.append_object(Box::new(IntObject(44))).unwrap();
        assert_same_int(&mem, int_obj2, 43);
        assert_same_int(&mem, int_obj3, 44);
    }

    fn assert_same_int(mem: &Memory, addr: Addr, expect: i64) {
        let returned_obj = &**mem.get_object(addr).unwrap();
        assert!(check_type::<IntObject>(returned_obj));
        let returned_int_obj = as_type::<IntObject>(returned_obj).unwrap();
        assert_eq!(*returned_int_obj, IntObject(expect));
    }

    #[test]
    fn random_hold() {
        extern crate rand;
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for _ in 0..10 {
            let mut mem = Memory::with_max_object_count(1024);
            let mut addr_vec = Vec::<Addr>::new();
            let mut alive_set = HashSet::<Addr>::new();
            let mut obj_count = 0;
            let root = mem.append_object(Box::new(IntObject(obj_count))).unwrap();
            mem.set_root(root);
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
                    mem.hold(holder_addr, obj);
                    if alive_set.contains(&holder_addr) {
                        alive_set.insert(obj);
                    }
                    chance -= 0.2;
                }
            }
            let _ = mem.append_object(Box::new(DummyObject)).unwrap();
            for addr in addr_vec {
                if alive_set.contains(&addr) {
                    assert!(mem.get_object(addr).is_some());
                } else {
                    assert!(mem.get_object(addr).is_none());
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
