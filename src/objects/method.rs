//

use crate::core::interp::{Interp, Name};
use crate::core::object::Object;

pub struct MethodObject {
    context: Name,
    pub(crate) run: fn(&mut Interp),
}

impl MethodObject {
    pub fn append_to(interp: &mut Interp, context: Name, run: fn(&mut Interp)) -> Option<Name> {
        let method = interp.append_object(Box::new(MethodObject { context, run }))?;
        interp.set_property(method, "context", context);
        Some(method)
    }
}

impl Object for MethodObject {
    fn get_property(&self, key: &str) -> Option<Name> {
        if key == "context" {
            Some(self.context)
        } else {
            None
        }
    }

    fn set_property(&mut self, key: &str, new_prop: Name) {
        assert_eq!(key, "context");
        assert_eq!(new_prop, self.context);
    }
}
