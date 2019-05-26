//

use crate::core::object::{AsMethod, AsProp, Object};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IntObject(pub i64);

impl Object for IntObject {}
impl AsMethod for IntObject {}
impl AsProp for IntObject {}
