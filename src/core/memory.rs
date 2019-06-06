//

use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::object::{Object, SharedObject};
use crate::core::runtime_error::RuntimeError;

pub enum QuasiObject {
    Local(Object),
    Shared(SharedObject),
}

pub type Addr = usize;

pub struct Memory {
    max_object_count: usize,
    objects: HashMap<Addr, QuasiObject>,
    next_addr: Addr,
    pub ref_map: RefMap,
}

pub struct RefMap {
    entry: Option<Addr>,
    graph: HashMap<Addr, HashSet<Addr>>,
}

impl Memory {
    pub fn new(count: usize) -> Self {
        Self {
            max_object_count: count,
            objects: HashMap::new(),
            next_addr: 0,
            ref_map: RefMap::new(),
        }
    }

    pub fn insert(&mut self, object: QuasiObject) -> Result<Addr, RuntimeError> {
        if self.objects.len() == self.max_object_count {
            self.collect();
        }

        if self.objects.len() == self.max_object_count {
            return Err(RuntimeError::MemoryFull);
        }
        let addr = self.next_addr;
        self.next_addr += 1;
        self.objects.insert(addr, object);
        self.ref_map.graph.insert(addr, HashSet::new());
        Ok(addr)
    }

    pub fn get(&self, addr: Addr) -> Result<&QuasiObject, RuntimeError> {
        self.objects
            .get(&addr)
            .ok_or_else(|| RuntimeError::SegFault)
    }

    pub fn get_mut(&mut self, addr: Addr) -> Result<&mut QuasiObject, RuntimeError> {
        self.objects
            .get_mut(&addr)
            .ok_or_else(|| RuntimeError::SegFault)
    }

    pub fn replace(&mut self, dest: Addr, src: QuasiObject) -> Result<QuasiObject, RuntimeError> {
        let replaced = self
            .objects
            .remove(&dest)
            .ok_or_else(|| RuntimeError::SegFault)?;
        self.objects.insert(dest.to_owned(), src);
        Ok(replaced)
    }
}

impl RefMap {
    fn new() -> Self {
        Self {
            graph: HashMap::new(),
            entry: None,
        }
    }

    pub fn set_entry(&mut self, addr: Addr) -> Result<(), RuntimeError> {
        self.entry = Some(addr);
        Ok(())
    }

    pub fn hold(&mut self, holder: Addr, holdee: Addr) -> Result<(), RuntimeError> {
        self.graph.get_mut(&holder).unwrap().insert(holdee);
        Ok(())
    }

    pub fn unhold(&mut self, holder: Addr, holdee: Addr) -> Result<(), RuntimeError> {
        self.graph.get_mut(&holder).unwrap().remove(&holdee);
        Ok(())
    }
}

impl Memory {
    pub fn collect(&mut self) {
        use std::time::Instant;
        let now = Instant::now();

        let mut queue = VecDeque::new();
        let mut dead_set: HashSet<_> = self.objects.keys().cloned().collect();
        if let Some(root_addr) = &self.ref_map.entry {
            queue.push_back(root_addr.to_owned());
        }

        // for each alive object
        while let Some(object_addr) = queue.pop_front() {
            // remove it from dead set
            dead_set.remove(&object_addr);
            // queue its holdee
            for holdee_addr in self.ref_map.graph[&object_addr].iter() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::object::{Object, Prop, To};

    #[derive(Debug, PartialEq, Eq)]
    struct Int(i32);

    impl Prop for Int {
        fn get(&self, _key: &str) -> Result<Addr, RuntimeError> {
            unimplemented!()
        }

        fn set(&mut self, _key: &str, _value: Addr) -> Result<(), RuntimeError> {
            unimplemented!()
        }
    }

    #[test]
    fn insert_local() {
        let mut mem = Memory::new(16);
        let object_id = mem
            .insert(QuasiObject::Local(Object::new(Int(42))))
            .unwrap();
        if let QuasiObject::Local(object) = mem.get(object_id).unwrap() {
            assert_eq!(object.to_ref::<Int>().unwrap(), &Int(42));
        }

        mem.collect();
        assert!(mem.get(object_id).is_err());
    }
}
