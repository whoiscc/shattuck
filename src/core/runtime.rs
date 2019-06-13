//

use crate::core::error::{Error, Result};
use crate::core::memory::{Address, Memory};
use crate::core::object::{GetHoldee, Object};

pub struct Runtime {
    pub memory: Memory,
    frame_stack: Vec<Address>,
}

struct Frame {
    context: Address,
}

unsafe impl GetHoldee for Frame {
    fn get_holdee(&self) -> Vec<Address> {
        vec![self.context]
    }
}

pub struct RuntimeBuilder {
    memory: Memory,
    context: Address,
}

impl RuntimeBuilder {
    pub fn new(memory: Memory, context: Address) -> Self {
        Self { memory, context }
    }

    pub fn boot(self) -> Result<Runtime> {
        let mut memory = self.memory;
        let first_frame = memory.insert_local(Object::new(Frame {
            context: self.context,
        }))?;
        memory.set_entry(first_frame);
        Ok(Runtime {
            memory,
            frame_stack: vec![first_frame],
        })
    }
}

impl Runtime {
    pub fn context(&self) -> Address {
        self.frame_stack
            .last()
            .unwrap()
            .get_ref()
            .unwrap()
            .as_ref::<Frame>()
            .unwrap()
            .context
    }
}

pub type MethodFn = fn(runtime: &mut Runtime) -> Result<()>;

#[derive(Clone, Copy)]
pub struct Method {
    context: Address,
    internal: MethodFn,
}

impl Method {
    pub fn new(internal: MethodFn, context: Address) -> Self {
        Self { context, internal }
    }

    pub fn bind(&self, new_context: Address) -> Self {
        let mut method = self.to_owned();
        method.context = new_context;
        method
    }
}

unsafe impl GetHoldee for Method {
    fn get_holdee(&self) -> Vec<Address> {
        vec![self.context]
    }
}

impl Runtime {
    pub fn call(&mut self, method: Address) -> Result<()> {
        let method_object = method
            .get_ref()?
            .as_ref::<Method>()
            .map_err(|_| Error::NotCallable)?
            .to_owned();
        let frame = self.memory.insert_local(Object::new(Frame {
            context: method_object.context,
        }))?;
        self.frame_stack.push(frame);
        self.memory.set_entry(frame);
        (method_object.internal)(self)?;
        self.frame_stack.pop();
        self.memory.set_entry(*self.frame_stack.last().unwrap());
        Ok(())
    }
}
