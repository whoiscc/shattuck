//

use std::any::Any;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::memory::Addr;
use crate::core::runtime_error::RuntimeError;

pub trait AnyProp: Any + GetProp + SetProp + AsAny + AsProp {}

impl<T: Any + GetProp + SetProp> AnyProp for T {}

pub trait AsAny {
    fn any_ref(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AnyProp> AsAny for T {
    fn any_ref(&self) -> &dyn Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait AsProp {
    fn get_prop_ref(&self) -> &dyn GetProp;
    fn get_prop_mut(&mut self) -> &mut dyn GetProp;
    fn set_prop_mut(&mut self) -> &mut dyn SetProp;
}

impl<T: AnyProp> AsProp for T {
    fn get_prop_ref(&self) -> &dyn GetProp {
        self
    }

    fn get_prop_mut(&mut self) -> &mut dyn GetProp {
        self
    }

    fn set_prop_mut(&mut self) -> &mut dyn SetProp {
        self
    }
}

pub trait GetProp {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError>;
}

pub trait SetProp {
    fn set(&mut self, key: &str, value: Addr) -> Result<(), RuntimeError>;
}

pub trait Prop {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError>;
    fn set(&mut self, key: &str, value: Addr) -> Result<(), RuntimeError>;
}

impl<T: Prop> GetProp for T {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError> {
        self.get(key)
    }
}

impl<T: Prop> SetProp for T {
    fn set(&mut self, key: &str, value: Addr) -> Result<(), RuntimeError> {
        self.set(key, value)
    }
}

pub trait To {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError>;
}

pub trait ToMut: To {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError>;
}

pub struct Object(Box<dyn AnyProp>);

impl Object {
    pub fn new<T: Any + Prop>(content: T) -> Self {
        Object(Box::new(content))
    }
}

impl To for Object {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        (&*self.0 as &dyn AnyProp)
            .any_ref()
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl ToMut for Object {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError> {
        (&mut *self.0 as &mut dyn AnyProp)
            .any_mut()
            .downcast_mut::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl GetProp for Object {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError> {
        (&*self.0 as &dyn AnyProp).get_prop_ref().get(key)
    }
}

impl SetProp for Object {
    fn set(&mut self, key: &str, value: Addr) -> Result<(), RuntimeError> {
        (&mut *self.0 as &mut dyn AnyProp)
            .set_prop_mut()
            .set(key, value)
    }
}

pub struct SharedObject(Arc<RwLock<dyn AnyProp + Send + Sync>>);

impl SharedObject {
    pub fn new<T: Any + Send + Sync + Prop>(content: T) -> Self {
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

    pub fn share(&self) -> SharedObject {
        SharedObject(Arc::clone(&self.0))
    }
}

pub struct ReadShared<'a>(RwLockReadGuard<'a, dyn AnyProp + Send + Sync>);

impl<'a> To for ReadShared<'a> {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        (&*self.0 as &dyn AnyProp)
            .any_ref()
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl<'a> GetProp for ReadShared<'a> {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError> {
        (&*self.0 as &dyn AnyProp).get(key)
    }
}

pub struct WriteShared<'a>(RwLockWriteGuard<'a, dyn AnyProp + Send + Sync>);

impl<'a> To for WriteShared<'a> {
    fn to_ref<T: Any>(&self) -> Result<&T, RuntimeError> {
        (&*self.0 as &dyn AnyProp)
            .any_ref()
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl<'a> ToMut for WriteShared<'a> {
    fn to_mut<T: Any>(&mut self) -> Result<&mut T, RuntimeError> {
        (&mut *self.0 as &mut dyn AnyProp)
            .any_mut()
            .downcast_mut::<T>()
            .ok_or(RuntimeError::TypeMismatch)
    }
}

impl<'a> GetProp for WriteShared<'a> {
    fn get(&self, key: &str) -> Result<Addr, RuntimeError> {
        (&*self.0 as &dyn AnyProp).get(key)
    }
}

impl<'a> SetProp for WriteShared<'a> {
    fn set(&mut self, key: &str, value: Addr) -> Result<(), RuntimeError> {
        (&mut *self.0 as &mut dyn AnyProp).set(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Int(i32);

    impl Prop for Int {
        fn get(&self, _key: &str) -> Result<Addr, RuntimeError> {
            unimplemented!()
        }

        fn set(&mut self, _key: &str, _value: Addr) -> Result<(), RuntimeError> {
            unimplemented!()
        }
    }

    #[test]
    fn simple_int() {
        let i0 = Object::new(Int(42));
        assert!(i0.to_ref::<i32>().is_err());
        assert_eq!(i0.to_ref::<Int>().unwrap(), &Int(42));
    }

    #[test]
    fn sample_shared_int() {
        let i0 = SharedObject::new(Int(42));
        assert_eq!(i0.read().unwrap().to_ref::<Int>().unwrap(), &Int(42));
        {
            let _i0_read = i0.read().unwrap();
            assert!(i0.write().is_err());
        }

        let i1 = i0.share();
        use std::thread;
        let handle = thread::spawn(move || {
            assert_eq!(i1.read().unwrap().to_ref::<Int>().unwrap(), &Int(42));
            *i1.write().unwrap().to_mut::<Int>().unwrap() = Int(43);
        });
        handle.join().unwrap();
        assert_eq!(i0.read().unwrap().to_ref::<Int>().unwrap(), &Int(43));
    }
}
