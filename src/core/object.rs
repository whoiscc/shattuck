//

use std::any::Any;

use crate::objects::method::MethodObject;
use crate::objects::prop::PropObject;

pub trait Object: Any + Send + Sync + AsAny + AsMethod + AsProp + CloneObject {
    //
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Object> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait AsMethod {
    fn as_method(&self) -> Option<&dyn MethodObject> {
        None
    }
}

pub trait AsProp {
    fn as_prop(&self) -> Option<&dyn PropObject> {
        None
    }

    fn as_prop_mut(&mut self) -> Option<&mut dyn PropObject> {
        None
    }
}

pub trait CloneObject {
    fn clone_object(&self) -> Option<Box<dyn Object>> {
        None
    }
}

impl<O> CloneObject for O
where
    O: Object + Clone,
{
    fn clone_object(&self) -> Option<Box<dyn Object>> {
        Some(Box::new(self.clone()))
    }
}
