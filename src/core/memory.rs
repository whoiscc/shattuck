//

use crate::core::runtime_error::RuntimeError;

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

pub struct Memory<O, A, G> {
    max_object_count: usize,
    objects: HashMap<A, O>,
    pub next_addr: G,
    pub ref_map: RefMap<A>,
}

pub struct RefMap<A> {
    entry: Option<A>,
    graph: HashMap<A, HashSet<A>>,
}

pub trait AddrGen {
    type Addr;
    fn create(&mut self) -> Self::Addr;
}

impl<O, A, G> Memory<O, A, G>
where
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn new(count: usize, addr_gen: G) -> Self {
        Memory {
            max_object_count: count,
            objects: HashMap::new(),
            next_addr: addr_gen,
            ref_map: RefMap::new(),
        }
    }

    pub fn insert(&mut self, object: O) -> Result<A, RuntimeError> {
        if self.objects.len() == self.max_object_count {
            self.collect();
        }

        if self.objects.len() == self.max_object_count {
            return Err(RuntimeError::MemoryFull);
        }
        let addr = self.next_addr.create();
        self.objects.insert(addr.clone(), object);
        self.ref_map.graph.insert(addr.clone(), HashSet::new());
        Ok(addr)
    }

    pub fn get(&self, addr: &A) -> Result<&O, RuntimeError> {
        self.objects.get(addr).ok_or_else(|| RuntimeError::SegFault)
    }

    pub fn get_mut(&mut self, addr: &A) -> Result<&mut O, RuntimeError> {
        self.objects
            .get_mut(addr)
            .ok_or_else(|| RuntimeError::SegFault)
    }

    pub fn replace(&mut self, dest: &A, src: O) -> Result<O, RuntimeError> {
        let replaced = self
            .objects
            .remove(dest)
            .ok_or_else(|| RuntimeError::SegFault)?;
        self.objects.insert(dest.to_owned(), src);
        Ok(replaced)
    }
}

impl<A> RefMap<A>
where
    A: Hash + Eq + Clone,
{
    fn new() -> Self {
        Self {
            graph: HashMap::new(),
            entry: None,
        }
    }

    pub fn set_entry(&mut self, addr: A) -> Result<(), RuntimeError> {
        self.graph.get(&addr).ok_or(RuntimeError::SegFault)?;
        self.entry = Some(addr);
        Ok(())
    }

    pub fn hold(&mut self, holder: &A, holdee: &A) -> Result<(), RuntimeError> {
        self.graph
            .get_mut(holder)
            .ok_or_else(|| RuntimeError::SegFault)?
            .insert(holdee.to_owned());
        Ok(())
    }

    pub fn unhold(&mut self, holder: &A, holdee: &A) -> Result<(), RuntimeError> {
        self.graph
            .get_mut(holder)
            .ok_or_else(|| RuntimeError::SegFault)?
            .remove(holdee);
        Ok(())
    }

    pub fn replace_hold(&mut self, holder: &A, old: &A, new: &A) -> Result<(), RuntimeError> {
        self.unhold(holder, old)?;
        self.hold(holder, new)?;
        Ok(())
    }
}

impl<O, A, G> Memory<O, A, G>
where
    A: Hash + Eq + Clone,
    G: AddrGen<Addr = A>,
{
    pub fn collect(&mut self) {
        use std::time::Instant;
        let now = Instant::now();

        let mut queue = VecDeque::<A>::new();
        let mut dead_set: HashSet<A> = self.objects.keys().cloned().collect();
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
    use crate::util::addr_gen::Inc;

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct Object(i64);

    #[test]
    fn test_insert() {
        let mut mem = Memory::new(16, Inc::new());
        let object_id = mem.insert(Object(42)).unwrap();
        assert_eq!(mem.get(&object_id).unwrap(), &Object(42));
    }
}
