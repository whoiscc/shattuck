//

extern crate shattuck;
use shattuck::core::memory::{Memory, Addr};
use shattuck::core::object::{AsProp, Object};
use shattuck::core::runtime::{make_shared, Arg, Runtime, RuntimeError};
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;
use shattuck::objects::thread::ThreadObject;

use std::thread::{current, JoinHandle};

#[derive(Clone)]
struct ThreadMethod;

impl AsProp for ThreadMethod {}

impl MethodObject for ThreadMethod {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError> {
        println!("I am running at {:?}!", current().id());
        let forty_two = runtime.append_object(Box::new(IntObject(42)))?;
        runtime.set_arg(Arg::insert(vec![forty_two], runtime.memory())?)?;
        Ok(())
    }
}

impl Object for ThreadMethod {}


fn main() {
    let mem = make_shared(Memory::new(128));
    let mut runtime = Runtime::new(mem).unwrap();
    let t0 = runtime.append_object(Box::new(ThreadMethod)).unwrap();
    let t1 = ThreadObject::insert(runtime.memory(), t0).unwrap();
    runtime.run_method(t1).unwrap();
    let t2 = runtime.index_arg(0).unwrap();
    let handle = runtime.replace_object(t2, Box::new(IntObject(42)));
}
