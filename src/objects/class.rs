//

use std::collections::HashMap;

use crate::core::memory::Address;

pub struct Class {
    props: HashMap<String, Address>,
}