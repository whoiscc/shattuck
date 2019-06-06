//
//

use std::any::Any;

use crate::core::memory::{Addr, Memory, QuasiObject};
use crate::core::object::{
    GetProp, Object, Prop, ReadShared, SetProp, SharedObject, To, ToMut, WriteShared,
};
use crate::core::runtime_error::RuntimeError;

pub struct Runtime {
    pub memory: Memory,
    frame_stack: Vec<Frame>,
}

impl Runtime {
    pub fn context(&self) -> Addr {
        self.frame_stack.last().unwrap().context
    }
}

pub enum ReadObject<'a> {
    Local(&'a Object),
    Shared(ReadShared<'a>),
}

impl<'a> To for ReadObject<'a> {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        match self {
            ReadObject::Local(object) => object.to_ref(),
            ReadObject::Shared(object) => object.to_ref(),
        }
    }
}

impl<'a> GetProp for ReadObject<'a> {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError> {
        match self {
            ReadObject::Local(object) => object.get(key),
            ReadObject::Shared(object) => object.get(key),
        }
    }
}

pub enum WriteObject<'a> {
    Local(&'a mut Object),
    Shared(WriteShared<'a>),
}

impl<'a> To for WriteObject<'a> {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        match self {
            WriteObject::Local(object) => object.to_ref(),
            WriteObject::Shared(object) => object.to_ref(),
        }
    }
}

impl<'a> ToMut for WriteObject<'a> {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError> {
        match self {
            WriteObject::Local(object) => object.to_mut(),
            WriteObject::Shared(object) => object.to_mut(),
        }
    }
}

impl<'a> GetProp for WriteObject<'a> {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError> {
        match self {
            WriteObject::Local(object) => object.get(key),
            WriteObject::Shared(object) => object.get(key),
        }
    }
}

impl<'a> SetProp for WriteObject<'a> {
    fn set(&mut self, key: &str, value: Addr) -> Result<(), RuntimeError> {
        match self {
            WriteObject::Local(object) => object.set(key, value),
            WriteObject::Shared(object) => object.set(key, value),
        }
    }
}

impl Runtime {
    pub fn read(&self, addr: Addr) -> Result<ReadObject, RuntimeError> {
        Ok(match self.memory.get(addr)? {
            QuasiObject::Local(object) => ReadObject::Local(object),
            QuasiObject::Shared(object) => ReadObject::Shared(object.read()?),
        })
    }

    pub fn write(&mut self, addr: Addr) -> Result<WriteObject, RuntimeError> {
        Ok(match self.memory.get_mut(addr)? {
            QuasiObject::Local(object) => WriteObject::Local(object),
            QuasiObject::Shared(object) => WriteObject::Shared(object.write()?),
        })
    }
}

pub struct Frame {
    context: Addr,
}

impl Runtime {
    pub fn new(memory: Memory, context: Addr) -> Self {
        Self {
            memory,
            frame_stack: vec![Frame { context }],
        }
    }
}

pub type MethodInternal = fn(&mut Runtime) -> Result<(), RuntimeError>;

#[derive(Clone)]
pub struct Method {
    internal: MethodInternal,
    context: Addr,
}

impl Method {
    pub fn insert_local(
        memory: &mut Memory,
        internal: MethodInternal,
        context: Addr,
    ) -> Result<Addr, RuntimeError> {
        let object = QuasiObject::Local(Object::new(Method { internal, context }));
        let addr = memory.insert(object)?;
        memory.ref_map.hold(addr, context)?;
        Ok(addr)
    }

    pub fn insert_shared(
        memory: &mut Memory,
        internal: MethodInternal,
        context: Addr,
    ) -> Result<Addr, RuntimeError> {
        let object = QuasiObject::Shared(SharedObject::new(Method { internal, context }));
        let addr = memory.insert(object)?;
        memory.ref_map.hold(addr, context)?;
        Ok(addr)
    }
}

impl Prop for Method {
    fn get(&self, _key: &str) -> Result<Addr, RuntimeError> {
        unimplemented!()
    }

    fn set(&mut self, _key: &str, _value: Addr) -> Result<(), RuntimeError> {
        unimplemented!()
    }
}

impl Runtime {
    pub fn call(&mut self, method: Addr) -> Result<(), RuntimeError> {
        let cloned_method;
        {
            let read = self.read(method)?;
            cloned_method = read.to_ref::<Method>()?.to_owned();
        }
        self.frame_stack.push(Frame {
            context: cloned_method.context,
        });
        (cloned_method.internal)(self)?;
        self.frame_stack.pop();
        Ok(())
    }
}
