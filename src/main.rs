//

extern crate shattuck;
use shattuck::core::interp::Interp;
use shattuck::objects::derived::DerivedObject;
use shattuck::objects::method::MethodObject;

fn dummy_method(_interp: &mut Interp) {
    println!("I am running!");
}

fn main() {
    let context = Box::new(DerivedObject::new());
    let mut interp = Interp::new(context, 128);
    let t1 = interp
        .append_object(Box::new(MethodObject(dummy_method)))
        .unwrap();
    interp.run_method(interp.context(), t1);
}
