//

use crate::core::object::Object;
use crate::core::runtime::Name;

#[derive(Debug, PartialEq, Eq)]
pub struct IntObject(pub i64);

impl Object for IntObject {
    fn get_property(&self, _key: &str) -> Option<Name> {
        panic!();
    }

    fn set_property(&mut self, _key: &str, _new_prop: Name) {
        panic!();
    }
}
