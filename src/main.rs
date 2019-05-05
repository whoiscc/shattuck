//

extern crate shattuck;
use shattuck::core::interp::{Interp, InterpError};
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

#[derive(Clone)]
struct DummyMethod;

impl MethodObject for DummyMethod {
    fn run(&self, interp: &mut Interp) -> Result<(), InterpError> {
        println!("I am running!");
        let context: &IntObject = interp.get_object(interp.context())?;
        println!("{:?}", context);
        Ok(())
    }
}

fn main() -> Result<(), InterpError> {
    let mut interp = Interp::new(128)?;
    let t0 = interp.append_object(Box::new(IntObject(42)))?;
    let t1 = interp.append_object(Box::new(DummyMethod))?;
    interp.set_context(t0);
    interp.run_method(t1)?;
    interp.garbage_collect();
    Ok(())
}
