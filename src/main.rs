//

extern crate shattuck;
use shattuck::core::runtime::RuntimeError;
use shattuck::core::shared_runtime::{with, SharedRuntime};
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

#[derive(Clone)]
struct DummyMethod;

impl MethodObject for DummyMethod {
    fn run(&self, runtime: &SharedRuntime) -> Result<(), RuntimeError> {
        println!("I am running!");
        let with_runtime = with(runtime);
        let context: &IntObject = with_runtime.get_object(with_runtime.context())?;
        println!("{:?}", context);
        Ok(())
    }
}

fn main() {
    let runtime = SharedRuntime::new(128).unwrap();
    let t0 = runtime
        .write()
        .append_object(Box::new(IntObject(42)))
        .unwrap();
    let t1 = runtime
        .write()
        .append_object(Box::new(DummyMethod))
        .unwrap();
    runtime.write().set_context(t0);
    runtime.run(t1).unwrap();
    runtime.write().garbage_collect();
}
