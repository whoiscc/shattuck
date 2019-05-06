//

use std::any::Any;

use crate::core::runtime::Name;
use crate::objects::method::MethodObject;

pub trait Object: Any + AsAny {
    fn get_property(&self, _key: &str) -> Option<Name> {
        None
    }

    fn set_property(&mut self, _key: &str, _new_prop: Name) {
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
