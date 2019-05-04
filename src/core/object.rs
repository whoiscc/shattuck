//

use std::any::Any;
use crate::core::memory::Addr;

pub trait Object: Any + AsAny {
    fn get_property(&self, key: &String) -> Option<Addr>;
    fn set_property(&mut self, key: &String, new_prop: Addr);
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
