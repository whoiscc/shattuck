//

use std::sync::Arc;

use crate::core::error::Error as ShattuckError;
use crate::core::object::Object;

use failure::Error;
use hulunbuir::{
    slot::{Slot, Take},
    Address, Collector as RawCollector, Keep,
};
use parking_lot::Mutex;

pub type Collector = Arc<Mutex<RawCollector<Slot<Object>>>>;

pub struct Runtime {
    memory: Collector,
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
        self.address_stack
            .pop()
            .ok_or(ShattuckError::ExhaustedFrame)?;
        Ok(())
    }

    fn get_address(&self, index: usize) -> Result<&Address, ShattuckError> {
        if index == 0 || self.address_stack.len() < index {
            return Err(ShattuckError::ExhaustedFrame);
        }
        Ok(self
            .address_stack
            .get(self.address_stack.len() - index)
            .unwrap())
    }

    fn stack_len(&self) -> usize {
        self.address_stack.len()
    }
}

pub struct RuntimeBuilder {
    collector: Collector,
    frame_object: Frame,
}

impl RuntimeBuilder {
    pub fn new(collector: Collector, context: Address) -> Self {
        let frame_object = Frame::new(context, None);
        Self {
            collector,
            frame_object,
        }
    }

    pub fn boot(self) -> Result<Runtime, Error> {
        let frame = self
            .collector
            .lock()
            .allocate(Slot::new(Object::new(self.frame_object)))?;
        Ok(Runtime {
            memory: self.collector,
            frame_stack: vec![frame],
        })
    }
}

impl Runtime {
    pub fn push(&mut self, object: Object) -> Result<(), Error> {
        let addr = self.memory.lock().allocate(Slot::new(object))?;
        self.with_current_frame_mut(|frame| frame.push_address(addr));
        Ok(())
    }

    pub fn pop(&mut self) -> Result<(), Error> {
        self.with_current_frame_mut(|frame| frame.pop_address().map_err(Into::into))
    }

    pub fn take(&mut self, index: usize) -> Result<Object, Error> {
        let addr = self.clone_address(index)?;
        match self.memory.lock().take(&addr)? {
            Take::Free(object) => Ok(object),
            Take::Busy(_) => Err(ShattuckError::BusyObject.into()),
        }
    }

    pub fn wait(&mut self, index: usize) -> Result<Object, Error> {
        let addr = self.clone_address(index)?;
        self.wait_object(&addr)
    }

    pub fn fill(&mut self, index: usize, object: Object) -> Result<(), Error> {
        let addr = self.clone_address(index)?;
        self.memory.lock().fill(&addr, object).map_err(Into::into)
    }

    fn clone_address(&mut self, index: usize) -> Result<Address, Error> {
        self.with_current_frame_mut(|frame| {
            frame
                .get_address(index)
                .map(ToOwned::to_owned)
                .map_err(Into::into)
        })
    }

    fn wait_object(&self, address: &Address) -> Result<Object, Error> {
        loop {
            let take = self.memory.lock().take(address)?;
            match take {
                Take::Free(object) => return Ok(object),
                Take::Busy(parker) => parker.park(),
            }
        }
    }

    fn with_current_frame_ref<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Frame) -> R,
    {
        let frame = self.frame_stack.last().unwrap().to_owned();
        let mut frame_object = self.wait_object(&frame).unwrap().downcast().unwrap();
        let result = f(&mut frame_object);
        self.memory
            .lock()
            .fill(&frame, Object::new(frame_object))
            .unwrap();
        result
    }

    fn with_current_frame_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Frame) -> R,
    {
        let frame = self.frame_stack.last().unwrap().to_owned();
        let mut frame_object = self.wait_object(&frame).unwrap().downcast().unwrap();
        let result = f(&mut frame_object);
        self.memory
            .lock()
            .fill(&frame, Object::new(frame_object))
            .unwrap();
        result
    }

    pub fn stack_len(&self) -> usize {
        self.with_current_frame_ref(|frame| frame.stack_len())
    }

    pub fn call(&mut self, context: usize, arguments: &[usize]) -> Result<(), Error> {
        let caller_frame = self.frame_stack.last().unwrap().to_owned();
        let callee_frame_object =
            self.with_current_frame_ref::<_, Result<_, Error>>(|caller_frame_object| {
                let context = caller_frame_object.get_address(context)?.to_owned();
                let mut frame = Frame::new(context, Some(caller_frame.clone()));
                for arg in arguments.iter().rev() {
                    let addr = caller_frame_object.get_address(*arg)?.to_owned();
                    frame.push_address(addr);
                }
                Ok(frame)
            })?;
        let callee_frame = self
            .memory
            .lock()
            .allocate(Slot::new(Object::new(callee_frame_object)))?;
        self.frame_stack.push(callee_frame);
        Ok(())
    }

    pub fn back(&mut self, returned: &[usize]) -> Result<(), Error> {
        if self.frame_stack.len() == 1 {
            return Err(ShattuckError::NoParentFrame.into());
        }
        let callee_frame = self.frame_stack.last().unwrap();
        let callee_frame_object: Frame =
            self.wait_object(callee_frame).unwrap().downcast().unwrap();
        self.frame_stack.pop().unwrap();
        self.with_current_frame_mut::<_, Result<_, Error>>(|caller_frame_object| {
            for ret in returned.iter().rev() {
                let addr = callee_frame_object.get_address(*ret)?.to_owned();
                caller_frame_object.push_address(addr);
            }
            Ok(())
        })?;
        Ok(())
    }
}
