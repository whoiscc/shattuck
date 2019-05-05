//

use std::any::Any;

use crate::core::interp::Name;

pub trait Object: Any + AsAny {
    fn get_property(&self, key: &str) -> Option<Name>;
    fn set_property(&mut self, key: &str, new_prop: Name);
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Object> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn as_type<T: 'static>(object: &dyn Object) -> Option<&T> {
    object.as_any().downcast_ref::<T>()
}

pub fn check_type<T: 'static>(object: &dyn Object) -> bool {
    object.as_any().is::<T>()
}
