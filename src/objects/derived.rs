//

use std::collections::HashMap;

use crate::core::memory::Addr;
use crate::core::object::{AsMethod, CloneObject, Object};
use crate::objects::prop::PropObject;

#[derive(Debug)]
pub struct DerivedObject {
    props: HashMap<String, Addr>,
}

impl DerivedObject {
    pub fn new() -> Self {
        DerivedObject {
            props: HashMap::new(),
        }
    }

    pub fn get_property(&self, key: &str) -> Option<Addr> {
        self.props.get(key).cloned()
    }

    pub fn set_property(&mut self, key: &str, new_prop: Addr) {
        self.props.insert(key.to_string(), new_prop);
    }
}

impl Default for DerivedObject {
    fn default() -> Self {
        DerivedObject::new()
    }
}

impl PropObject for DerivedObject {
    fn get_prop(&self, key: &str) -> Option<Addr> {
        self.get_property(key)
    }

    fn set_prop(&mut self, key: &str, prop: Addr) {
        self.set_property(key, prop)
    }
}

impl AsMethod for DerivedObject {}

impl Object for DerivedObject {}

impl CloneObject for DerivedObject {}
