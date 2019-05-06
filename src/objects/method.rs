//

use crate::core::runtime::{Runtime, RuntimeError};
use crate::core::object::Object;

pub trait MethodObject: Object {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError>;
}

impl<T: 'static + MethodObject + Clone> Object for T {
    fn as_method(&self) -> Option<Box<dyn MethodObject>> {
        Some(Box::new(self.clone()))
    }
}
