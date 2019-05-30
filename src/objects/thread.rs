//

use crate::core::memory::Addr;
use crate::core::object::{AsMethod, AsProp, CloneObject, Object};
use crate::core::runtime::{Arg, Memory, Runtime, RuntimeError};
use crate::objects::method::MethodObject;

use std::thread::{spawn, JoinHandle};

// memory address of method executing in newly created thread
#[derive(Clone)]
pub struct ThreadObject(Addr);

impl ThreadObject {
    pub fn insert(mem: Memory, method: Addr) -> Result<Addr, RuntimeError> {
        let thd = ThreadObject(method);
        let thd_addr = mem.write().unwrap().append_object(Box::new(thd))?;
        mem.write().unwrap().hold(thd_addr, method)?;
        Ok(thd_addr)
    }
}

impl MethodObject for ThreadObject {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError> {
        let thread_memory = runtime.memory();
        let thread_args = runtime.arg();
        let ThreadObject(method) = *self;
        let mut thread_runtime = Runtime::new(thread_memory)?;
        thread_runtime.set_arg(thread_args)?;
        let handle = spawn(move || {
            thread_runtime.run_method(method).unwrap(); // TODO
            thread_runtime.arg()
        });
        let join_object = runtime.append_object(Box::new(JoinObject(handle)))?;
        runtime.set_arg(Arg::insert(vec![join_object], runtime.memory())?)?;
        Ok(())
    }
}

impl AsProp for ThreadObject {}
impl Object for ThreadObject {}

pub struct JoinObject(pub JoinHandle<Addr>);

impl AsProp for JoinObject {}
impl AsMethod for JoinObject {}
impl CloneObject for JoinObject {}
impl Object for JoinObject {}
