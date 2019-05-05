//

use crate::core::interp::{Interp, InterpError};
use crate::core::object::Object;

pub trait MethodObject: Object {
    fn run(&self, interp: &mut Interp) -> Result<(), InterpError>;
}

impl<T: 'static + MethodObject + Clone> Object for T {
    fn as_method(&self) -> Option<Box<dyn MethodObject>> {
        Some(Box::new(self.clone()))
    }
}
