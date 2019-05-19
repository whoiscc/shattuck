//

use crate::core::object::Object;
use crate::core::runtime::RuntimeError;
use crate::core::shared_runtime::SharedRuntime;

pub trait MethodObject: Object {
    fn run(&self, runtime: &SharedRuntime) -> Result<(), RuntimeError>;
}

impl<T: 'static + MethodObject + Clone> Object for T {
    fn as_method(&self) -> Option<Box<dyn MethodObject>> {
        Some(Box::new(self.clone()))
    }
}
