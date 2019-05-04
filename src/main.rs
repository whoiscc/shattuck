//

extern crate shattuck;
use shattuck::core::memory::Memory;
use shattuck::objects::{DerivedObject, IntObject};

fn main() {
    let mut mem = Memory::with_max_object_count(3);
    // yukari = new DerivedObject()
    let yukari = mem.append_object(Box::new(DerivedObject::new())).unwrap();
    // age = new IntObject(18)
    let age = mem.append_object(Box::new(IntObject(18))).unwrap();
    // yukari.age = age
    let key = "age".to_string();
    mem.set_object_property(yukari, &key, age);
    // print(yukari.age)
    let age_prop = mem.get_object_property(yukari, &key);
    println!("{:?}", age_prop);
    println!("{:?}", mem.get_object(age_prop.unwrap()));

    // marisa = new DerivedObject()
    let marisa = mem.append_object(Box::new(DerivedObject::new())).unwrap();
    mem.set_root(marisa);
    // correct_age = new IntObject(4294967296)
    let correct_age = mem.append_object(Box::new(IntObject(4294967296))).unwrap();
    println!("{:?}", mem.set_object_property(yukari, &key, correct_age));
}
