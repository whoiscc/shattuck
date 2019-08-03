//

use std::any::Any;

use crate::core::error::Error;

use hulunbuir::{Keep, Address};

pub struct Object {
    content: Box<dyn Any>,
    keep: fn(&Object) -> Vec<Address>,
}

impl Keep for Object {
    fn with_keep<F: FnOnce(&[Address])>(&self, f: F) {
        f(&(self.keep)(self))
    }
}

fn keep_helper<T: Any + Keep>(object: &Object) -> Vec<Address> {
    let mut keep_list = Vec::new();
    object.downcast_ref::<T>().unwrap().with_keep(|list| keep_list = list.to_vec());
    keep_list
}

impl Object {
    pub fn new<T: Any + Keep>(content: T) -> Self {
        Object {
            content: Box::new(content),
            keep: keep_helper::<T>,
        }
    }
}

impl Object {
    pub fn downcast_ref<T: Any>(&self) -> Result<&T, Error> {
        (&self.content as &dyn Any).downcast_ref().ok_or(Error::TypeMismatch)
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Result<&mut T, Error> {
        (&mut self.content as &mut dyn Any).downcast_mut().ok_or(Error::TypeMismatch)
    }

    pub fn downcast<T: Any>(self) -> Result<T, Error> {
        Ok(*self.content.downcast().map_err(|_| Error::TypeMismatch)?)
    }
}