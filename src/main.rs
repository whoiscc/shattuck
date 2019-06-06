//

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::vec::IntoIter;

extern crate shattuck;
use shattuck::core::runtime::{Builder, IntoShared, Runtime};
use shattuck::core::runtime_error::RuntimeError;
use shattuck::object::{AsObject, Object, SharedObject};
use shattuck::util::addr_gen::Inc;

#[derive(Debug)]
struct IntObject(i64);

type I = IntoIter<usize>;

impl IntoShared<Shared<I>, I> for IntObject {
    fn into_shared(self) -> Result<(Shared<I>, I), RuntimeError> {
        Ok((Shared(Box::new(self), PhantomData), Vec::new().into_iter()))
    }
}

impl AsObject<Shared<I>, I> for IntObject {
    fn as_object(&self) -> &dyn Object<Shared<I>, I> {
        self
    }

    fn as_object_mut(&mut self) -> &mut dyn Object<Shared<I>, I> {
        self
    }
}

impl Object<Shared<I>, I> for IntObject {}
impl SharedObject<Shared<I>, I> for IntObject {}

struct Local<I>(Box<dyn Object<Shared<I>, I>>, PhantomData<I>);
struct Shared<I>(Box<dyn SharedObject<Shared<I>, I>>, PhantomData<I>);

impl Deref for Local<I> {
    type Target = dyn Object<Shared<I>, I>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for Local<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl Deref for Shared<I> {
    type Target = dyn Object<Shared<I>, I>;

    fn deref(&self) -> &Self::Target {
        &*self.0.as_object()
    }
}

impl DerefMut for Shared<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0.as_object_mut()
    }
}

use std::thread;

fn main() {
    //
}
