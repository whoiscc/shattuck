//

use crate::core::object::{AsMethod, Object};
use crate::core::runtime::{Runtime, RuntimeError};

pub trait MethodObject: Object {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError>;
}

impl<O> AsMethod for O
where
    O: 'static + Clone + MethodObject,
{
    fn as_method(&self) -> Option<&dyn MethodObject> {
        Some(self)
    }
}
