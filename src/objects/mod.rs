//

use std::collections::HashMap;

use crate::core::object::Object;
use crate::core::memory::Addr;


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

    pub fn get_property(&self, key: &String) -> Option<Addr> {
        self.props.get(key).cloned()
    }

    pub fn set_property(&mut self, key: &String, new_prop: Addr) {
        // TODO: old prop checking
        self.props.insert(key.clone(), new_prop);
    }
}

impl Object for DerivedObject {
    fn get_property(&self, key: &String) -> Option<Addr> {
        self.get_property(key)
    }

    fn set_property(&mut self, key: &String, new_prop: Addr) {
        self.set_property(key, new_prop)
    }
}

#[derive(Debug)]
pub struct IntObject(pub i64);

impl Object for IntObject {
    fn get_property(&self, _key: &String) -> Option<Addr> {
        panic!();
    }

    fn set_property(&mut self, _key: &String, _new_prop: Addr) {
        panic!();
    }
}
