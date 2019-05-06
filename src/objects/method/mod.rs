//

use crate::core::object::Object;
use crate::core::runtime::{Runtime, RuntimeError};

pub trait MethodObject: Object {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError>;
}

impl<T: 'static + MethodObject + Clone> Object for T {
    fn as_method(&self) -> Option<Box<dyn MethodObject>> {
        Some(Box::new(self.clone()))
    }
}
