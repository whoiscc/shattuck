//

use crate::core::runtime_error::RuntimeError;
use crate::util::Inc;

use std::collections::{HashMap, HashSet, VecDeque};

pub struct Memory<O> {
    max_object_count: usize,
    // the space actually saved (the pointers of) the objects
    objects: HashMap<usize, O>,
    // counter for usize and Privusize
    next_addr: Inc,
    // root index in `to_space`
    root_addr: Option<usize>,
    // reference map, keys and values are indices of `to_space`
    ref_map: HashMap<usize, HashSet<usize>>,
}

impl<O> Memory<O> {
    pub fn new(count: usize) -> Self {
        Memory {
            max_object_count: count,
            objects: HashMap::new(),
            next_addr: Inc::new(),
            root_addr: None,
            ref_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, object: O) -> Result<usize, RuntimeError> {
        if self.objects.len() == self.max_object_count {
            self.collect();
        }

        if self.objects.len() == self.max_object_count {
            return Err(RuntimeError::MemoryFull);
        }
        let addr = self.next_addr.create();
        self.objects.insert(addr, object);
        self.ref_map.insert(addr, HashSet::new());
        Ok(addr)
    }

    pub fn get(&self, addr: usize) -> Result<&O, RuntimeError> {
        self.objects
            .get(&addr)
            .ok_or_else(|| RuntimeError::SegFault)
    }

    pub fn get_mut(&mut self, addr: usize) -> Result<&mut O, RuntimeError> {
        self.objects
            .get_mut(&addr)
            .ok_or_else(|| RuntimeError::SegFault)
    }

    pub fn replace(&mut self, dest: usize, src: O) -> Result<O, RuntimeError> {
        let replaced = self
            .objects
            .remove(&dest)
            .ok_or_else(|| RuntimeError::SegFault)?;
        self.objects.insert(dest, src);
        Ok(replaced)
    }

    pub fn set_root(&mut self, addr: usize) -> Result<(), RuntimeError> {
        self.get(addr)?;
        self.root_addr = Some(addr);
        Ok(())
    }

    pub fn hold(&mut self, holder: usize, holdee: usize) -> Result<(), RuntimeError> {
        self.get(holdee)?;
        self.ref_map
            .get_mut(&holder)
            .ok_or_else(|| RuntimeError::SegFault)?
            .insert(holdee);
        Ok(())
    }

    pub fn unhold(&mut self, holder: usize, holdee: usize) -> Result<(), RuntimeError> {
        self.get(holdee)?;
        self.ref_map
            .get_mut(&holder)
            .ok_or_else(|| RuntimeError::SegFault)?
            .remove(&holdee);
        Ok(())
    }

    pub fn replace_hold(
        &mut self,
        holder: usize,
        old: usize,
        new: usize,
    ) -> Result<(), RuntimeError> {
        self.unhold(holder, old)?;
        self.hold(holder, new)?;
        Ok(())
    }

    pub fn collect(&mut self) {
        use std::time::Instant;
        let now = Instant::now();

        let mut queue = VecDeque::<usize>::new();
        let mut dead_set: HashSet<usize> = self.objects.keys().cloned().collect();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct Object(i64);

    #[test]
    fn test_insert() {
        let mut mem = Memory::new(16);
        let object_id = mem.insert(Object(42)).unwrap();
        assert_eq!(mem.get(object_id).unwrap(), &Object(42));
    }
}
