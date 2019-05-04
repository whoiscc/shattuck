//

use std::fmt::Debug;
use crate::core::memory::Addr;

pub trait Object: Debug {
    fn get_property(&self, key: &String) -> Option<Addr>;
    fn set_property(&mut self, key: &String, new_prop: Addr);
}
