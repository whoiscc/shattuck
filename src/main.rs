//

extern crate shattuck;
use shattuck::core::interp::Interp;
use shattuck::objects::derived::DerivedObject;
use shattuck::objects::int::IntObject;
use shattuck::objects::method::MethodObject;

fn dummy_method(interp: &mut Interp) {
    println!("I am running!");
    let context: Option<&IntObject> = interp.get_object(interp.context());
    println!("{:?}", context);
}

fn main() {
    let context = Box::new(DerivedObject::new());
    let mut interp = Interp::new(context, 128);
    let t0 = interp.append_object(Box::new(IntObject(42))).unwrap();
    let t1 = MethodObject::append_to(&mut interp, t0, dummy_method).unwrap();
    interp.run_method(t1);
}
