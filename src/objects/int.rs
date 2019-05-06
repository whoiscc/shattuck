//

use crate::core::object::Object;
use crate::core::runtime::Pointer;

#[derive(Debug, PartialEq, Eq)]
pub struct IntObject(pub i64);

impl Object for IntObject {
    fn get_property(&self, _key: &str) -> Option<Pointer> {
        panic!();
    }

    fn set_property(&mut self, _key: &str, _new_prop: Pointer) {
        panic!();
    }
}
