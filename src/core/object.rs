//

use std::any::Any;

use crate::core::runtime::Pointer;
use crate::objects::method::MethodObject;

pub trait Object: Any + AsAny {
    fn get_property(&self, _key: &str) -> Option<Pointer> {
        None
    }

    fn set_property(&mut self, _key: &str, _new_prop: Pointer) {
        //
    }

    // downcast
    fn as_method(&self) -> Option<Box<dyn MethodObject>> {
        None
    }
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Object> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
