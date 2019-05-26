//

use crate::core::memory::Addr;
use crate::core::object::{AsProp, Object};

pub trait PropObject: Object {
    fn get_prop(&self, key: &str) -> Option<Addr>;
    fn set_prop(&mut self, key: &str, prop: Addr);
}

impl<O> AsProp for O
where
    O: PropObject,
{
    fn as_prop(&self) -> Option<&dyn PropObject> {
        Some(self)
    }

    fn as_prop_mut(&mut self) -> Option<&mut dyn PropObject> {
        Some(self)
    }
}
