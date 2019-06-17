//

use std::thread::sleep;
use std::time::Duration;

extern crate shattuck;
use shattuck::core::error::Result;
use shattuck::core::memory::Memory;
use shattuck::core::object::{Object, ToSync, Orphan};
use shattuck::core::runtime::{Method, RuntimeBuilder};
use shattuck::objects::thread::make_thread;

extern crate rand;
use rand::random;

#[derive(Debug)]
struct Int(i32);

unsafe impl Orphan for Int {}

impl ToSync for Int {
    type Target = Int;

    fn to_sync(self) -> Result<Self::Target> {
        Ok(self)
    }
}

fn main() {
    let mut memory = Memory::new(16);
    let context = memory.insert_local(Object::new(Int(42))).unwrap();
    let mut runtime = RuntimeBuilder::new(memory, context).boot().unwrap();
    // method = (x) -> this += x
    // return nothing
    let method_object = Method::new(
        |runtime| {
            sleep(Duration::from_millis(
                (random::<u64>() % 1000 + 1000) % 1000,
            ));
            let mut get_context = runtime.context();
            let mut get_mut_this = get_context.get_mut()?;
            let this = get_mut_this.as_ref::<Int>()?.0;
            let x = runtime.get(1)?.get_ref()?.as_ref::<Int>()?.0;
            get_mut_this.as_mut::<Int>()?.0 = this + x;
            Ok(())
        },
        runtime.context(),
    );

    let method = runtime
        .memory
        .insert_local(Object::new(method_object))
        .unwrap();
    let x = runtime.memory.insert_local(Object::new(Int(1))).unwrap();
    // method(x)
    runtime.push(x);
    runtime.call(method, &[1]).unwrap();
    println!(
        "{:?}",
        runtime
            .context()
            .get_ref()
            .unwrap()
            .as_local_ref::<Int>()
            .unwrap()
    );

    // thread = $make_thread(method)
    // $make_thread will be replaced by Thread.new after introducing classes
    let thread_object = make_thread(method);
    let thread = runtime
        .memory
        .insert_local(Object::new(thread_object))
        .unwrap();
    // join = thread(x)
    runtime.call(thread, &[1]).unwrap();
    let join = runtime.get(1).unwrap();

    sleep(Duration::from_millis(
        (random::<u64>() % 1000 + 1000) % 1000,
    ));
    println!(
        "43 or 44 or panic? {:?}",
        runtime
            .context()
            .get_ref()
            .unwrap()
            .as_shared_ref::<Int>()
            .unwrap()
    );

    // <result> = join()
    // no result here
    runtime.call(join, &[]).unwrap();

    println!(
        "{:?}",
        runtime
            .context()
            .get_ref()
            .unwrap()
            .as_ref::<Int>()
            .unwrap()
    );
}
