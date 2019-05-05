//

use crate::core::interp::{Interp, Name};
use crate::core::object::Object;

pub struct MethodObject(pub fn(&mut Interp));

impl Object for MethodObject {
    fn get_property(&self, _key: &str) -> Option<Name> {
        None
    }

    fn set_property(&mut self, _key: &str, _new_prop: Name) {
        //
    }
}
