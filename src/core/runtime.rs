//

use crate::core::error::Error as ShattuckError;
use crate::core::object::Object;

use hulunbuir::{Collector, Address, Keep};
use failure::Error;

pub struct Runtime {
    memory: Collector<Object>,
    frame_stack: Vec<Address>,
}

struct Frame {
    context: Address,
    address_stack: Vec<Address>,
    parent: Option<Address>,
}

impl Keep for Frame {
    fn with_keep<F: FnMut(&[Address])>(&self, mut f: F) {
        f(&[self.context.to_owned()]);
        f(&self.address_stack);
        if let Some(addr) = &self.parent {
            f(&[addr.to_owned()]);
        }
    }
}

impl Frame {
    fn new(context: Address, parent: Option<Address>) -> Self {
        Self {
            context,
            address_stack: Vec::new(),
            parent,
        }
    }

    fn push_address(&mut self, address: Address) {
        self.address_stack.push(address);
    }

    fn pop_address(&mut self) -> Result<(), ShattuckError> {
        self.address_stack.pop().ok_or(ShattuckError::ExhaustedFrame)?;
        Ok(())
    }

    fn get_address(&self, index: usize) -> Result<Address, ShattuckError> {
        if self.address_stack.len() < index {
            return Err(ShattuckError::ExhaustedFrame);
        }
        Ok(self
            .address_stack
            .get(self.address_stack.len() - index)
            .unwrap()
            .to_owned())
    }
}
