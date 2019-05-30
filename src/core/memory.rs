//

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq)]
pub struct Addr(usize);

pub struct Memory<O> {
    max_object_count: usize,
    // the space actually saved (the pointers of) the objects
    objects: HashMap<Addr, O>,
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

impl<O> Memory<O> {
    pub fn new(count: usize) -> Self {
        Memory {
            max_object_count: count,
            objects: HashMap::new(),
            next_addr: 0,
            root_addr: None,
            ref_map: HashMap::new(),
        }
    }

    pub fn append_object(&mut self, object: O) -> Result<Addr, MemoryError> {
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

    pub fn get_object(&self, addr: Addr) -> Result<&O, MemoryError> {
        self.objects
            .get(&addr)
            .ok_or_else(|| MemoryError::InvalidAddr(addr))
    }

    pub fn get_object_mut(&mut self, addr: Addr) -> Result<&mut O, MemoryError> {
        self.objects
            .get_mut(&addr)
            .ok_or_else(|| MemoryError::InvalidAddr(addr))
    }

    pub fn replace_object(&mut self, dest: Addr, src: O) -> Result<O, MemoryError> {
        let replaced = self
            .objects
            .remove(&dest)
            .ok_or_else(|| MemoryError::InvalidAddr(dest))?;
        self.objects.insert(dest, src);
        Ok(replaced)
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

    pub fn unhold(&mut self, holder: Addr, holdee: Addr) -> Result<(), MemoryError> {
        self.get_object(holdee)?;
        self.ref_map
            .get_mut(&holder)
            .ok_or_else(|| MemoryError::InvalidAddr(holder))?
            .remove(&holdee);
        Ok(())
    }

    pub fn replace_hold(&mut self, holder: Addr, old: Addr, new: Addr) -> Result<(), MemoryError> {
        self.unhold(holder, old)?;
        self.hold(holder, new)?;
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
}

#[cfg(test)]
mod tests {
    //
}
