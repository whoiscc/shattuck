//

use std::collections::HashMap;

use crate::core::object::Object;
use crate::core::interp::Name;


#[derive(Debug)]
pub struct DerivedObject {
    props: HashMap<String, Name>,
}

impl DerivedObject {
    pub fn new() -> Self {
        DerivedObject {
            props: HashMap::new(),
        }
    }

    pub fn get_property(&self, key: &str) -> Option<Name> {
        self.props.get(key).cloned()
    }

    pub fn set_property(&mut self, key: &str, new_prop: Name) {
        // TODO: old prop checking
        self.props.insert(key.to_string(), new_prop);
    }
}

impl Object for DerivedObject {
    fn get_property(&self, key: &str) -> Option<Name> {
        self.get_property(key)
    }

    fn set_property(&mut self, key: &str, new_prop: Name) {
        self.set_property(key, new_prop)
    }
}

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
