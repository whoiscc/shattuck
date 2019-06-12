//

use std::any::Any;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::error::{Error, Result};
use crate::core::memory::Address;

type GetObjectHoldee = fn(&Object) -> Vec<Address>;

pub unsafe trait GetHoldeeOfObject {
    fn get_holdee(object: &Object) -> Vec<Address>;
}

pub struct Object {
    content: Box<dyn Any>,
    get_holdee_f: GetObjectHoldee,
}

impl Object {
    pub fn new<T: Any + GetHoldeeOfObject>(content: T) -> Self {
        Self {
            content: Box::new(content),
            get_holdee_f: T::get_holdee,
        }
    }

    pub fn as_ref<T: Any>(&self) -> Result<&T> {
        self.content.downcast_ref().ok_or(Error::TypeMismatch)
    }

    pub fn as_mut<T: Any>(&mut self) -> Result<&mut T> {
        self.content.downcast_mut().ok_or(Error::TypeMismatch)
    }

    pub fn take<T: Any>(self) -> Result<T> {
        let content = *self
            .content
            .downcast::<T>()
            .map_err(|_| Error::TypeMismatch)?;
        Ok(content)
    }

    pub fn get_holdee(&self) -> Vec<Address> {
        (self.get_holdee_f)(self)
    }
}

type GetSyncObjectHoldee = fn(&SyncObject) -> Vec<Address>;

pub unsafe trait GetHoldeeOfSyncObject {
    fn get_holdee(object: &SyncObject) -> Vec<Address>;
}

#[derive(Clone)]
pub struct SyncObject {
    content: Arc<RwLock<dyn Any + Send + Sync>>,
    get_holdee_f: GetSyncObjectHoldee,
}

impl SyncObject {
    pub fn new<T: Any + Send + Sync + GetHoldeeOfSyncObject>(content: T) -> Self {
        Self {
            content: Arc::new(RwLock::new(content)),
            get_holdee_f: T::get_holdee,
        }
    }

    pub fn get_holdee(&self) -> Vec<Address> {
        (self.get_holdee_f)(self)
    }
}

pub trait ToSync {
    type Target: Any + Send + Sync + GetHoldeeOfSyncObject;
    fn to_sync(self) -> Result<Self::Target>;
}

impl Object {
    // explicit different name with ToSync::to_sync
    pub fn into_sync<T: Any + ToSync>(self) -> Result<SyncObject> {
        Ok(SyncObject::new(self.take::<T>()?.to_sync()?))
    }
}

pub struct SyncRef<'a>(RwLockReadGuard<'a, dyn Any + Send + Sync>);
pub struct SyncMut<'a>(RwLockWriteGuard<'a, dyn Any + Send + Sync>);

impl SyncObject {
    pub fn get_ref(&self) -> Result<SyncRef> {
        Ok(SyncRef(
            self.content.try_read().map_err(|_| Error::ViolateSync)?,
        ))
    }
}

impl<'a> SyncRef<'a> {
    pub fn as_ref<T: Any>(&self) -> Result<&T> {
        self.0.downcast_ref().ok_or(Error::TypeMismatch)
    }
}

impl SyncObject {
    pub fn get_mut(&self) -> Result<SyncMut> {
        Ok(SyncMut(
            self.content.try_write().map_err(|_| Error::ViolateSync)?,
        ))
    }
}

impl<'a> SyncMut<'a> {
    pub fn as_ref<T: Any>(&self) -> Result<&T> {
        self.0.downcast_ref().ok_or(Error::TypeMismatch)
    }

    pub fn as_mut<T: Any>(&mut self) -> Result<&mut T> {
        self.0.downcast_mut().ok_or(Error::TypeMismatch)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::mem;
    use std::sync::{Arc, RwLock};
    use std::thread;

    use super::*;

    struct TheAnswer(RefCell<Option<i32>>);

    impl TheAnswer {
        fn new() -> Self {
            Self(RefCell::new(None))
        }

        fn clear_cache(&mut self) {
            self.0 = RefCell::new(None);
        }

        fn ask_internal(&self) -> i32 {
            42
        }

        fn get(&self) -> i32 {
            let mut cache = self.0.borrow_mut();
            if let None = &*cache {
                *cache = Some(self.ask_internal());
            }
            mem::drop(cache);
            self.0.borrow().unwrap()
        }
    }

    struct SyncAnswer(RwLock<Option<i32>>);

    impl SyncAnswer {
        fn new() -> Self {
            Self(RwLock::new(None))
        }

        fn clear_cache(&mut self) {
            self.0 = RwLock::new(None);
        }

        fn ask_internal(&self) -> i32 {
            42
        }

        fn get(&self) -> i32 {
            let mut cache = self.0.write().unwrap();
            if let None = &*cache {
                *cache = Some(self.ask_internal());
            }
            mem::drop(cache);
            self.0.read().unwrap().unwrap()
        }
    }

    #[test]
    fn get_local_answer() {
        let mut ans = TheAnswer::new();
        assert_eq!(ans.get(), 42);
        ans.clear_cache();
        assert_eq!(ans.get(), 42);
    }

    #[test]
    fn get_sync_answer() {
        let mut ans = SyncAnswer::new();
        assert_eq!(ans.get(), 42);
        ans.clear_cache();
        assert_eq!(ans.get(), 42);

        let shared_ans = Arc::new(ans);
        let thread_ans = Arc::clone(&shared_ans);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                assert_eq!(thread_ans.get(), 42);
            }
        });
        for _ in 0..100 {
            assert_eq!(shared_ans.get(), 42);
        }
        handle.join().unwrap();
    }

    unsafe impl GetHoldeeOfObject for TheAnswer {
        fn get_holdee(_object: &Object) -> Vec<Address> {
            Vec::new()
        }
    }

    unsafe impl GetHoldeeOfSyncObject for SyncAnswer {
        fn get_holdee(_object: &SyncObject) -> Vec<Address> {
            Vec::new()
        }
    }

    #[test]
    fn create_object() {
        let mut ans_object = Object::new(TheAnswer::new());
        assert_eq!(ans_object.as_ref::<TheAnswer>().unwrap().get(), 42);
        ans_object.as_mut::<TheAnswer>().unwrap().clear_cache();
        assert_eq!(ans_object.as_ref::<TheAnswer>().unwrap().get(), 42);
    }

    impl ToSync for TheAnswer {
        type Target = SyncAnswer;

        fn to_sync(self) -> Result<Self::Target> {
            Ok(SyncAnswer(RwLock::new(self.0.into_inner())))
        }
    }

    #[test]
    fn take_inner() {
        let ans_object = Object::new(TheAnswer::new());
        assert_eq!(ans_object.take::<TheAnswer>().unwrap().get(), 42);
    }

    #[test]
    fn to_sync() {
        let ans_object = Object::new(TheAnswer::new());
        let sync_ans = ans_object.into_sync::<TheAnswer>().unwrap();
        assert_eq!(
            sync_ans
                .get_ref()
                .unwrap()
                .as_ref::<SyncAnswer>()
                .unwrap()
                .get(),
            42
        );
    }
}
