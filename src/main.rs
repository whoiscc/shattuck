//

extern crate shattuck;
use shattuck::core::runtime::RuntimeError;
use shattuck::core::shared_runtime::{with, with_mut, SharedRuntime};
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

#[derive(Clone)]
struct DummyMethod;

impl MethodObject for DummyMethod {
    fn run(&self, runtime: &SharedRuntime) -> Result<(), RuntimeError> {
        println!("I am running!");

        // test borrow mut
        with_mut(runtime)?.push_env()?;

        {
            let with_runtime = with(runtime)?;
            let context: &IntObject = with_runtime.get_object(with_runtime.context())?;
            println!("{:?}", context);
        }

        with_mut(runtime)?.pop_env()?;

        Ok(())
    }
}

fn main() {
    let runtime = SharedRuntime::new(128).unwrap();
    let t0 = runtime
        .write()
        .unwrap()
        .append_object(Box::new(IntObject(42)))
        .unwrap();
    let t1 = runtime
        .write()
        .unwrap()
        .append_object(Box::new(DummyMethod))
        .unwrap();
    runtime.write().unwrap().set_context(t0);
    runtime.run(t1).unwrap();
    runtime.write().unwrap().garbage_collect();
}
