//

use std::any::Any;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::error::{Error, Result};
use crate::core::memory::Address;

type GetObjectHoldee = fn(&Object) -> Vec<Address>;

pub unsafe trait GetHoldee {
    fn get_holdee(&self) -> Vec<Address>;
}

trait GetHoldeeOfObject {
    fn get_object_holdee(object: &Object) -> Vec<Address>;
}

impl<T: Any + GetHoldee> GetHoldeeOfObject for T {
    fn get_object_holdee(object: &Object) -> Vec<Address> {
        // it is safe to `unwrap` here
        // this method will be related to correct type
        // in the constructor of `Object`
        object.as_ref::<Self>().unwrap().get_holdee()
    }
}

trait MakeSync {
    fn make_sync(object: Object) -> Result<SyncObject>;
}

impl<T: Any + ToSync> MakeSync for T {
    fn make_sync(object: Object) -> Result<SyncObject> {
        Ok(SyncObject::new(object.take::<T>()?.to_sync()?))
    }
}

type MakeSyncFn = fn(Object) -> Result<SyncObject>;

pub struct Object {
    content: Box<dyn Any>,
    get_holdee_f: GetObjectHoldee,
    make_sync_f: MakeSyncFn,
}

impl Object {
    pub fn new<T: Any + GetHoldee + ToSync>(content: T) -> Self {
        Self {
            content: Box::new(content),
            get_holdee_f: T::get_object_holdee,
            make_sync_f: T::make_sync,
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

trait GetHoldeeOfSyncObject {
    fn get_sync_object_holdee(object: &SyncObject) -> Vec<Address>;
}

impl<T: Any + GetHoldee> GetHoldeeOfSyncObject for T {
    fn get_sync_object_holdee(object: &SyncObject) -> Vec<Address> {
        // it is safe to `unwrap` the result of `get_ref` here
        // this method will only be called by `Memory::collect`,
        // which keeps a mutable reference to the whole `Memory`
        // so no reference (of any kind) of this object could exist
        // at the same time
        object
            .get_ref()
            .unwrap()
            .as_ref::<Self>()
            .unwrap()
            .get_holdee()
    }
}

#[derive(Clone)]
pub struct SyncObject {
    content: Arc<RwLock<dyn Any + Send + Sync>>,
    get_holdee_f: GetSyncObjectHoldee,
}

impl SyncObject {
    pub fn new<T: Any + Send + Sync + GetHoldee>(content: T) -> Self {
        Self {
            content: Arc::new(RwLock::new(content)),
            get_holdee_f: T::get_sync_object_holdee,
        }
    }

    pub fn get_holdee(&self) -> Vec<Address> {
        (self.get_holdee_f)(self)
    }
}

pub trait ToSync {
    type Target: Any + Send + Sync + GetHoldee;
    fn to_sync(self) -> Result<Self::Target>;
}

impl Object {
    // explicit different name with ToSync::to_sync
    pub fn into_sync(self) -> Result<SyncObject> {
        (self.make_sync_f)(self)
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

    unsafe impl GetHoldee for TheAnswer {
        fn get_holdee(&self) -> Vec<Address> {
            Vec::new()
        }
    }

    unsafe impl GetHoldee for SyncAnswer {
        fn get_holdee(&self) -> Vec<Address> {
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
        let sync_ans = ans_object.into_sync().unwrap();
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
