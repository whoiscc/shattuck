//

use std::collections::HashMap;

use crate::core::object::Object;
use crate::core::runtime::Pointer;

#[derive(Debug)]
pub struct DerivedObject {
    props: HashMap<String, Pointer>,
}

impl DerivedObject {
    pub fn new() -> Self {
        DerivedObject {
            props: HashMap::new(),
        }
    }

    pub fn get_property(&self, key: &str) -> Option<Pointer> {
        self.props.get(key).cloned()
    }

    pub fn set_property(&mut self, key: &str, new_prop: Pointer) {
        self.props.insert(key.to_string(), new_prop);
    }
}

impl Default for DerivedObject {
    fn default() -> Self {
        DerivedObject::new()
    }
}

impl Object for DerivedObject {
    fn get_property(&self, key: &str) -> Option<Pointer> {
        self.get_property(key)
    }

    fn set_property(&mut self, key: &str, new_prop: Pointer) {
        self.set_property(key, new_prop)
    }
}
