//

extern crate shattuck;
use shattuck::core::interp::Interp;
use shattuck::objects::derived::DerivedObject;
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

#[derive(Clone)]
struct DummyMethod;

impl MethodObject for DummyMethod {
    fn run(&self, interp: &mut Interp) {
        println!("I am running!");
        let context: Option<&IntObject> = interp.get_object(interp.context());
        println!("{:?}", context);
    }
}

fn main() {
    let context = Box::new(DerivedObject::new());
    let mut interp = Interp::new(context, 128);
    let t0 = interp.append_object(Box::new(IntObject(42))).unwrap();
    let t1 = interp.append_object(Box::new(DummyMethod)).unwrap();
    interp.set_context(t0);
    interp.run_method(t1);
}
