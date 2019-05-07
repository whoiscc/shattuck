//

use std::cell::RefCell;
use crate::core::object::Object;
use crate::core::runtime::{RuntimeManager, RuntimeError};

pub trait MethodObject: Object {
    fn run(&self, manager: &RefCell<RuntimeManager>) -> Result<(), RuntimeError>;
}

impl<T: 'static + MethodObject + Clone> Object for T {
    fn as_method(&self) -> Option<Box<dyn MethodObject>> {
        Some(Box::new(self.clone()))
    }
}
