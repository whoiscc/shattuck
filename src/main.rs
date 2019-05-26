//

extern crate shattuck;
use shattuck::core::memory::Memory;
use shattuck::core::object::{AsProp, Object};
use shattuck::core::runtime::{make_shared, Runtime, RuntimeError};
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

#[derive(Clone)]
struct DummyMethod;

impl AsProp for DummyMethod {}

impl MethodObject for DummyMethod {
    fn run(&self, runtime: &mut Runtime) -> Result<(), RuntimeError> {
        println!("I am running!");

        // test borrow mut
        runtime.push_env()?;

        println!(
            "{:?}",
            runtime.get_object(runtime.context()).to::<IntObject>()?
        );

        runtime.pop_env()?;

        Ok(())
    }
}

impl Object for DummyMethod {}

fn main() {
    let mem = make_shared(Memory::new(128));
    let mut runtime = Runtime::new(mem).unwrap();
    let t0 = runtime.append_object(Box::new(IntObject(42))).unwrap();
    let t1 = runtime.append_object(Box::new(DummyMethod)).unwrap();
    runtime.set_context(t0);
    runtime.run_method(t1).unwrap();
    runtime.garbage_collect();
}
