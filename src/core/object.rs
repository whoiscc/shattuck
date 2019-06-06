//

use std::any::Any;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::runtime_error::RuntimeError;

pub struct Object(Box<dyn Any>);

pub trait To {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError>;
}

pub trait ToMut: To {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError>;
}

impl Object {
    pub fn new<T: Any>(content: T) -> Self {
        Object(Box::new(content))
    }
}

impl To for Object {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        (&*self.0 as &dyn Any)
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl ToMut for Object {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError> {
        (&mut *self.0 as &mut dyn Any)
            .downcast_mut::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

pub struct SharedObject(Arc<RwLock<dyn Any + Send + Sync>>);

impl SharedObject {
    pub fn new<T: Any + Send + Sync>(content: T) -> Self {
        SharedObject(Arc::new(RwLock::new(content)))
    }

    pub fn read(&self) -> Result<ReadShared, RuntimeError> {
        Ok(ReadShared(
            self.0
                .try_read()
                .map_err(|_| RuntimeError::AccessConflict)?,
        ))
    }

    pub fn write(&self) -> Result<WriteShared, RuntimeError> {
        Ok(WriteShared(
            self.0
                .try_write()
                .map_err(|_| RuntimeError::AccessConflict)?,
        ))
    }
}

pub struct ReadShared<'a>(RwLockReadGuard<'a, dyn Any + Send + Sync>);

impl<'a> To for ReadShared<'a> {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        (&*self.0 as &dyn Any)
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

pub struct WriteShared<'a>(RwLockWriteGuard<'a, dyn Any + Send + Sync>);

impl<'a> To for WriteShared<'a> {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        (&*self.0 as &dyn Any)
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl<'a> ToMut for WriteShared<'a> {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError> {
        (&mut *self.0 as &mut dyn Any)
            .downcast_mut::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_int() {
        let i0 = Object::new(42);
        assert!(i0.to_ref::<bool>().is_err());
        assert_eq!(i0.to_ref::<i32>().unwrap(), &42);
    }

    #[test]
    fn sample_shared_int() {
        let i0 = SharedObject::new(42);
        assert_eq!(i0.read().unwrap().to_ref::<i32>().unwrap(), &42);
        let _i0_read = i0.read().unwrap();
        assert!(i0.write().is_err());

        //
    }
}
