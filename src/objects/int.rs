//

use crate::core::object::Object;

#[derive(Debug, PartialEq, Eq)]
pub struct IntObject(pub i64);

impl Object for IntObject {}
