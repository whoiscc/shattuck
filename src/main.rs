//

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::vec::IntoIter;

extern crate shattuck;
use shattuck::core::runtime::{Builder, IntoShared, Runtime};
use shattuck::core::runtime_error::RuntimeError;
use shattuck::object::Object;
use shattuck::util::addr_gen::Inc;

#[derive(Debug)]
struct IntObject(i64);

impl Object for IntObject {}

struct Local<A>(Box<dyn Object>, PhantomData<A>);
struct Shared(Box<dyn Object>);

impl<A> Deref for Local<A> {
    type Target = dyn Object;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<A> DerefMut for Local<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl Deref for Shared {
    type Target = dyn Object;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for Shared {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl<A> IntoShared<Shared> for Local<A> {
    type Iter = IntoIter<A>;

    fn into_shared(self) -> Result<(Shared, Self::Iter), RuntimeError> {
        Ok((Shared(self.0), Vec::<A>::new().into_iter()))
    }
}

fn main() {
    let mut builder: Builder<Local<_>, Shared, _, _> = Builder::new(16, Inc::new());
    let addr = builder
        .insert_local(Local(Box::new(IntObject(42)), PhantomData))
        .unwrap();
    let mut runtime = Runtime::new(builder, addr);
    {
        let object = runtime.read(&addr).unwrap();
        println!("{:?}", object.as_any().downcast_ref::<IntObject>());
    }
    let _ = runtime.share(&addr);
    {
        let object = runtime.read(&addr).unwrap();
        println!("{:?}", object.as_any().downcast_ref::<IntObject>());
    }
}
