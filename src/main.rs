//

extern crate shattuck;
use shattuck::core::interp::Interp;
use shattuck::core::object::as_type;
use shattuck::objects::{DerivedObject, IntObject};

fn main() {
    let context = Box::new(DerivedObject::new());
    let mut interp = Interp::new(context, 128);
    interp.push_frame();
    interp.push_env();
    // let yukari = new DerivedObject()
    interp.insert_object("yukari", Box::new(DerivedObject::new()));
    // this.wife = yukari  # prevent yukari to be collected
    interp
        .set_context_object_property("wife", "yukari")
        .unwrap();
    // let age = new IntObject(18)
    interp.insert_object("age", Box::new(IntObject(18)));
    // yukari.age = age
    interp.set_object_property("yukari", "age", "age").unwrap();
    // print(this.wife.age)
    let backup_this = interp.get_context_object();
    // 1. this = this.wife
    interp.change_context_object(
        interp
            .get_object_property_raw(interp.get_context_object(), "wife")
            .unwrap(),
    );
    // 2. print(this.age)
    println!(
        "{:?}",
        as_type::<IntObject>(
            &**interp
                .get_raw_object(
                    interp
                        .get_object_property_raw(interp.get_context_object(), "age")
                        .unwrap()
                )
                .unwrap()
        )
    );
    // 3. restore this
    interp.change_context_object(backup_this);

    interp.garbage_collect();

    interp.pop_env();
    interp.push_env();

    // this.wife = new DerivedObject()
    interp.insert_object("random_wife", Box::new(DerivedObject::new()));
    interp.set_context_object_property("wife", "random_wife").unwrap();

    interp.garbage_collect();
}
