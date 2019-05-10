//

extern crate shattuck;
use shattuck::core::runtime::{Runtime, RuntimeError};
use shattuck::core::runtime_pool::RuntimePool;
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

#[derive(Clone)]
struct DummyMethod;

impl MethodObject for DummyMethod {
    fn run(&self, interp: &mut Runtime) -> Result<(), RuntimeError> {
        println!("I am running!");
        let context: &IntObject = interp.get_object(interp.context())?;
        println!("{:?}", context);
        Ok(())
    }
}

fn main() {
    let mut pool = RuntimePool::new();
    let runtime_id = pool.create_runtime(128).unwrap();
    let mut borrowed_runtime = pool.borrow_mut(runtime_id).unwrap();
    let t0 = borrowed_runtime
        .append_object(Box::new(IntObject(42)))
        .unwrap();
    let t1 = borrowed_runtime
        .append_object(Box::new(DummyMethod))
        .unwrap();
    borrowed_runtime.set_context(t0);
    borrowed_runtime.run_method(t1).unwrap();
    borrowed_runtime.garbage_collect();
}
