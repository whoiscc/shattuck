//

use crate::core::error::{Error, Result};
use crate::core::memory::{Address, Memory};
use crate::core::object::{GetHoldee, NoSync, Object, SyncObject, ToSync};

pub struct Runtime {
    pub memory: Memory,
    frame_stack: Vec<Address>,
}

struct Frame {
    context: Address,
    address_stack: Vec<Address>,
    parent: Option<Address>,
}

impl NoSync for Frame {}

unsafe impl GetHoldee for Frame {
    fn get_holdee(&self) -> Vec<Address> {
        let mut holdee_list = self.address_stack.to_owned();
        holdee_list.push(self.context);
        if let Some(addr) = self.parent {
            holdee_list.push(addr);
        }
        holdee_list
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

    fn pop_address(&mut self) -> Result<()> {
        self.address_stack.pop().ok_or(Error::ExhaustedFrame)?;
        Ok(())
    }

    fn get_address(&self, index: usize) -> Result<Address> {
        if self.address_stack.len() < index {
            return Err(Error::ExhaustedFrame);
        }
        Ok(self
            .address_stack
            .get(self.address_stack.len() - index)
            .unwrap()
            .to_owned())
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
        let first_frame = memory.insert_local(Object::new(Frame::new(self.context, None)))?;
        memory.set_entry(first_frame);
        Ok(Runtime {
            memory,
            frame_stack: vec![first_frame],
        })
    }
}

impl Runtime {
    fn with_current<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(&Frame) -> R,
    {
        callback(
            self.frame_stack
                .last()
                .unwrap()
                .get_ref()
                .unwrap()
                .as_ref::<Frame>()
                .unwrap(),
        )
    }

    fn with_current_mut<F, R>(&mut self, callback: F) -> R
    where
        F: FnOnce(&mut Frame) -> R,
    {
        callback(
            self.frame_stack
                .last_mut()
                .unwrap()
                .get_mut()
                .unwrap()
                .as_mut::<Frame>()
                .unwrap(),
        )
    }

    pub fn context(&self) -> Address {
        self.with_current(|frame| frame.context)
    }

    pub fn push(&mut self, address: Address) {
        self.with_current_mut(|frame| frame.push_address(address));
    }

    pub fn pop(&mut self) -> Result<()> {
        self.with_current_mut(|frame| frame.pop_address())
    }

    pub fn get(&self, index: usize) -> Result<Address> {
        self.with_current(|frame| frame.get_address(index))
    }

    pub fn len(&self) -> usize {
        self.with_current(|frame| frame.address_stack.len())
    }

    pub fn push_parent(&mut self, index: usize) -> Result<()> {
        let frame_count = self.frame_stack.len();
        if frame_count == 1 {
            return Err(Error::NoParentFrame);
        }
        let address = self.get(index)?;
        self.frame_stack
            .get_mut(frame_count - 2)
            .unwrap()
            .get_mut()
            .unwrap()
            .as_mut::<Frame>()
            .unwrap()
            .push_address(address);
        Ok(())
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

impl ToSync for Method {
    type Target = SyncMethod;

    fn to_sync(mut self) -> Result<Self::Target> {
        let context = self.context.share()?;
        Ok(SyncMethod {
            context,
            internal: self.internal,
        })
    }
}

#[derive(Clone)]
pub struct SyncMethod {
    context: SyncObject,
    internal: MethodFn,
}

impl Runtime {
    pub fn call(&mut self, method: Address, args: &[usize]) -> Result<usize> {
        let (context, internal) = method.get_ref()?.as_dual_ref(
            |local_method: &Method| Ok((local_method.context, local_method.internal)),
            |shared_method: &SyncMethod| {
                Ok((
                    self.memory
                        .insert_shared(shared_method.context.to_owned())?,
                    shared_method.internal,
                ))
            },
        )?;
        let current_frame_size = self.len();
        let mut frame_object = Frame::new(context, Some(*self.frame_stack.last().unwrap()));
        for index in args.iter() {
            frame_object.push_address(self.get(*index)?);
        }
        let frame = self.memory.insert_local(Object::new(frame_object))?;
        self.frame_stack.push(frame);
        self.memory.set_entry(frame);
        internal(self)?;
        self.frame_stack.pop();
        self.memory.set_entry(*self.frame_stack.last().unwrap());
        Ok(self.len() - current_frame_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;

    unsafe impl GetHoldee for Dummy {
        fn get_holdee(&self) -> Vec<Address> {
            Vec::new()
        }
    }

    impl ToSync for Dummy {
        type Target = Dummy;

        fn to_sync(self) -> Result<Self::Target> {
            Ok(self)
        }
    }

    struct Int(i32);

    unsafe impl GetHoldee for Int {
        fn get_holdee(&self) -> Vec<Address> {
            Vec::new()
        }
    }

    impl ToSync for Int {
        type Target = Int;

        fn to_sync(self) -> Result<Self::Target> {
            Ok(self)
        }
    }

    #[test]
    fn address_stack() {
        let mut memory = Memory::new(16);
        let context = memory.insert_local(Object::new(Dummy)).unwrap();
        let mut runtime = RuntimeBuilder::new(memory, context).boot().unwrap();
        let variable = runtime.memory.insert_local(Object::new(Int(42))).unwrap();
        runtime.push(variable);
        assert_eq!(
            runtime
                .get(1)
                .unwrap()
                .get_ref()
                .unwrap()
                .as_local_ref::<Int>()
                .unwrap()
                .0,
            42
        );
        let method_object = Method::new(
            |runtime| {
                assert!(runtime.get(1).is_err());
                Ok(())
            },
            runtime.context(),
        );
        let method = runtime
            .memory
            .insert_local(Object::new(method_object))
            .unwrap();
        runtime.call(method, &[]).unwrap();
        assert_eq!(
            runtime
                .get(1)
                .unwrap()
                .get_ref()
                .unwrap()
                .as_local_ref::<Int>()
                .unwrap()
                .0,
            42
        );
        assert!(runtime.get(2).is_err());
        runtime.pop().unwrap();
        assert!(runtime.get(1).is_err());
        assert!(runtime.pop().is_err());
    }

    #[test]
    fn call_method() {
        let mut memory = Memory::new(16);
        let context = memory.insert_local(Object::new(Dummy)).unwrap();
        let mut runtime = RuntimeBuilder::new(memory, context).boot().unwrap();
        let context = runtime.memory.insert_local(Object::new(Int(42))).unwrap();
        let method_object = Method::new(
            |runtime| {
                let a = runtime.get(1)?.get_ref()?.as_ref::<Int>()?.0;
                let b = runtime.context().get_ref()?.as_ref::<Int>()?.0;
                let c = runtime.memory.insert_local(Object::new(Int(a + b)))?;
                runtime.push(c);
                runtime.push_parent(1)?;
                Ok(())
            },
            context,
        );
        let method = runtime
            .memory
            .insert_local(Object::new(method_object))
            .unwrap();
        let arg = runtime.memory.insert_local(Object::new(Int(1))).unwrap();
        runtime.push(arg);
        runtime.call(method, &[1]).unwrap();
        assert_eq!(
            runtime
                .get(1)
                .unwrap()
                .get_ref()
                .unwrap()
                .as_ref::<Int>()
                .unwrap()
                .0,
            43
        );
    }
}
