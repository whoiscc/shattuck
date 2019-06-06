//
//

use std::any::Any;

use crate::core::memory::{Addr, Memory, QuasiObject};
use crate::core::object::{Object, ReadShared, To, ToMut, WriteShared};
use crate::core::runtime_error::RuntimeError;

pub struct Runtime {
    memory: Memory,
    frame_stack: Vec<Frame>,
}

impl Runtime {
    pub fn insert(&mut self, object: QuasiObject) -> Result<Addr, RuntimeError> {
        self.memory.insert(object)
    }

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
