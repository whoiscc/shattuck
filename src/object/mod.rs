//

use std::any::Any;

use crate::core::runtime::IntoShared;

// pub mod thread;

pub trait Object<S, I>: Any + AsAny<S, I> + IntoShared<S, I> {}

pub trait AsAny<S, I> {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<O, S, I> AsAny<S, I> for O
where
    O: Object<S, I>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait SharedObject<S, I>: Object<S, I> + AsObject<S, I> + Sync + Send {}

pub trait AsObject<S, I> {
    fn as_object(&self) -> &dyn Object<S, I>;
    fn as_object_mut(&mut self) -> &mut dyn Object<S, I>;
}

impl<S, I> AsObject<S, I> for S
where
    S: SharedObject<S, I>,
{
    fn as_object(&self) -> &dyn Object<S, I> {
        self
    }

    fn as_object_mut(&mut self) -> &mut dyn Object<S, I> {
        self
    }
}
