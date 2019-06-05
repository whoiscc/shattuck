//

use std::any::Any;

// pub mod thread;

pub trait Object: Any + AsAny {}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<O> AsAny for O
where
    O: Object,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
