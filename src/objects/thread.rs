//

use crate::core::memory::Addr;
use crate::core::object::{AsProp, Object};
use crate::core::runtime::{Runtime, RuntimeError};
use crate::objects::method::MethodObject;

use std::thread::Thread;

// memory address of method executing in newly created thread
#[derive(Clone)]
pub struct ThreadObject(Addr);

impl MethodObject for ThreadObject {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError> {
        Ok(())
    }
}

impl AsProp for ThreadObject {}

impl Object for ThreadObject {}
