//

extern crate shattuck;
use shattuck::core::runtime::{Runtime, RuntimeError, RuntimeManager};
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;
use std::cell::RefCell;
use std::thread;

#[derive(Clone)]
struct DummyMethod;

impl MethodObject for DummyMethod {
    fn run(&self, manager: &RefCell<RuntimeManager>) -> Result<(), RuntimeError> {
        println!("I am running!");
        let borrowed_manager = manager.borrow();
        let runtime = borrowed_manager.get(thread::current().id()).borrow();
        let context: &IntObject = runtime.get_object(runtime.context())?;
        println!("{:?}", context);
        Ok(())
    }
}

fn main() -> Result<(), RuntimeError> {
    let manager = RefCell::new(RuntimeManager::new());
    manager.borrow_mut().create(128)?;

    let borrowed_manager = manager.borrow();
    let runtime = borrowed_manager.get(thread::current().id());
    let t0 = runtime
        .borrow_mut()
        .append_object(Box::new(IntObject(42)))?;
    let t1 = runtime.borrow_mut().append_object(Box::new(DummyMethod))?;
    runtime.borrow_mut().set_context(t0);
    Runtime::run_method(&manager, t1)?;
    runtime.borrow_mut().garbage_collect();
    Ok(())
}
