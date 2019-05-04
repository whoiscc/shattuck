//

use std::collections::{HashMap, HashSet, VecDeque};
use std::mem::swap;

use crate::core::object::Object;

#[derive(Clone, Copy, Hash, Debug)]
pub struct Addr(usize);

impl PartialEq for Addr {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Addr {}

// prevent confusing from public Addr
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct PrivAddr(Addr);

pub struct Memory {
    max_object_count: usize,
    // the space actually saved (the pointers of) the objects
    objects: HashMap<PrivAddr, Box<dyn Object>>,
    // the indices of `objects`
    object_indices: Vec<PrivAddr>,
    // the indices of `object_indices`
    addr_map: HashMap<Addr, usize>,
    // counter for Addr and PrivAddr
    next_addr: usize,
    next_priv_addr: usize,
    // root index in `to_space`
    root_index: Option<usize>,
    // reference map, keys and values are indices of `to_space`
    ref_map: HashMap<usize, HashSet<usize>>,
}

impl Memory {
    pub fn with_max_object_count(count: usize) -> Self {
        Memory {
            max_object_count: count,
            objects: HashMap::new(),
            object_indices: Vec::new(),
            addr_map: HashMap::new(),
            next_addr: 0,
            next_priv_addr: 0,
            root_index: None,
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
        let priv_addr = self.allocate_priv_addr();
        self.objects.insert(priv_addr, object);
        let index = self.object_indices.len();
        self.object_indices.push(priv_addr);
        let addr = self.allocate_addr();
        self.addr_map.insert(addr, index);
        self.ref_map.insert(index, HashSet::new());
        Some(addr)
    }

    pub fn get_object(&self, addr: Addr) -> Option<&Box<dyn Object>> {
        let index = self.get_to_space_index(addr)?;
        let priv_addr = self.object_indices.get(index)?;
        let object = self.objects.get(priv_addr)?;
        Some(object)
    }

    pub fn set_root(&mut self, addr: Addr) {
        let index = self.get_to_space_index(addr).unwrap();
        assert!(self.object_indices.get(index).is_some());
        self.root_index = Some(index);
    }

    pub fn hold(&mut self, holder: Addr, holdee: Addr) {
        let holder_index = self.get_to_space_index(holder).unwrap();
        let holdee_index = self.get_to_space_index(holdee).unwrap();
        self.ref_map
            .get_mut(&holder_index)
            .unwrap()
            .insert(holdee_index);
    }

    pub fn drop(&mut self, holder: Addr, holdee: Addr) {
        let holder_index = self.get_to_space_index(holder).unwrap();
        let holdee_index = self.get_to_space_index(holdee).unwrap();
        self.ref_map
            .get_mut(&holder_index)
            .unwrap()
            .remove(&holdee_index);
    }

    fn collect(&mut self) {
        use std::time::Instant;
        let now = Instant::now();

        let mut alive_indices = Vec::<PrivAddr>::new();
        let mut queue = VecDeque::<usize>::new();
        if let Some(root_index) = self.root_index {
            queue.push_back(root_index);
        }
        let mut forward_map = HashMap::<usize, usize>::new();

        // for each alive object
        while let Some(object_index) = queue.pop_front() {
            // add it to alive list
            let priv_addr = self.object_indices[object_index];
            let new_index = alive_indices.len();
            alive_indices.push(priv_addr);
            forward_map.insert(object_index, new_index);
            // queue its holdee
            for holdee_index in self.ref_map[&object_index].iter() {
                if !forward_map.contains_key(holdee_index) {
                    queue.push_back(holdee_index.to_owned());
                }
            }
        }

        // update object_indices
        let mut old_indices = alive_indices;
        swap(&mut old_indices, &mut self.object_indices);
        // update addr_map, ref_map & objects
        let mut old_addr_map = HashMap::<Addr, usize>::new();
        let mut old_ref_map = HashMap::<usize, HashSet<usize>>::new();
        swap(&mut self.addr_map, &mut old_addr_map);
        swap(&mut self.ref_map, &mut old_ref_map);
        let (mut alive_count, mut dead_count) = (0, 0);
        for (addr, index) in old_addr_map.into_iter() {
            if forward_map.contains_key(&index) {
                let new_index = forward_map[&index];
                self.addr_map.insert(addr, new_index);
                let ref_set: HashSet<usize> = old_ref_map
                    .get(&index)
                    .take()
                    // each `index` appear only one in this for loop,
                    // so it's safe to assume `.get` always returns a `Some`
                    .unwrap()
                    .iter()
                    .map(|holdee| forward_map[holdee])
                    .collect();
                self.ref_map.insert(new_index, ref_set);
                alive_count += 1;
            } else {
                self.objects.remove(&old_indices[index]);
                dead_count += 1;
            }
        }
        // update root_index
        self.root_index = self
            .root_index
            .as_ref()
            .map(|root_index| forward_map[root_index]);

        assert!(alive_count + dead_count == self.max_object_count);
        println!(
            "<shattuck> garbage collected, {} alive, {} dead, duration: {} ms",
            alive_count, dead_count, now.elapsed().as_micros() as f64 / 1000.0
        );
    }

    fn get_to_space_index(&self, addr: Addr) -> Option<usize> {
        let index = self.addr_map.get(&addr)?;
        Some(index.to_owned())
    }

    fn allocate_addr(&mut self) -> Addr {
        let addr = Addr(self.next_addr);
        self.next_addr += 1;
        addr
    }

    fn allocate_priv_addr(&mut self) -> PrivAddr {
        let priv_addr = PrivAddr(Addr(self.next_priv_addr));
        self.next_priv_addr += 1;
        priv_addr
    }

    // object operation
    pub fn get_object_property(&mut self, addr: Addr, key: &String) -> Option<Addr> {
        self.get_object(addr)?.get_property(key)
    }

    pub fn set_object_property(&mut self, addr: Addr, key: &String, new_prop: Addr) -> Option<()> {
        let index = self.get_to_space_index(addr)?;
        let priv_addr = self.object_indices.get(index)?.to_owned();
        let object = self.objects.get(&priv_addr)?;
        if let Some(old_prop) = object.get_property(key) {
            self.drop(addr, old_prop);
        }
        let object_mut = self.objects.get_mut(&priv_addr)?;
        object_mut.set_property(key, new_prop);
        self.hold(addr, new_prop);
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyObject;
    impl Object for DummyObject {
        fn get_property(&self, _key: &String) -> Option<Addr> {
            panic!()
        }

        fn set_property(&mut self, _key: &String, _new_prop: Addr) {
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
