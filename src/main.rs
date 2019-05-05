//

extern crate shattuck;
use shattuck::core::interp::Interp;
use shattuck::objects::{DerivedObject, IntObject};

fn main() {
    let context = Box::new(DerivedObject::new());
    let mut interp = Interp::new(context, 128);
    interp.push_frame();
    interp.push_env();
    // let laptop = new DerivedObject()
    // 1. <t1> = new DerivedObject()
    let t1 = interp.append_object(Box::new(DerivedObject::new())).unwrap();
    // 2. let laptop = <t1>
    interp.insert_name(t1, "laptop");

    // laptop.size = new IntObject(13)
    // 1. <t2> = new IntObject(13)
    let t2 = interp.append_object(Box::new(IntObject(13))).unwrap();
    // 2. <t1>.size = <t2>
    interp.set_property(t1, "size", t2);

    // this.laptop = laptop
    // 1. <t3> = this
    let t3 = interp.context();
    // 2. <t3>.laptop = <t1>
    interp.set_property(t3, "laptop", t1);

    // laptop.size = new IntObject(15)
    // 1. <t4> = new IntObject(15)
    let t4 = interp.append_object(Box::new(IntObject(15))).unwrap();
    // 2. <t1>.size = <t4>
    interp.set_property(t1, "size", t4);

    // print(this.laptop.size)
    let t5 = interp.context();  // unnecessary
    let t6 = interp.get_property(t5, "laptop").unwrap();  // unnecessary
    let t7 = interp.get_property(t6, "size").unwrap();  // unnecessary
    println!("{:?}", interp.get_object::<IntObject>(t7));

    // IntObject(13) should be collected
    interp.garbage_collect();
}
