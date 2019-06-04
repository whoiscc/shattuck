//

use crate::core::memory::AddrGen;

#[derive(Default)]
pub struct Inc(usize);

impl Inc {
    pub fn new() -> Self {
        Inc(0)
    }

    pub fn create(&mut self) -> usize {
        let Self(next_id) = self;
        let id = *next_id;
        *next_id += 1;
        id
    }

    pub fn next_id(&self) -> usize {
        self.0
    }
}

impl AddrGen for Inc {
    type Addr = usize;

    fn create(&mut self) -> Self::Addr {
        self.create()
    }
}
