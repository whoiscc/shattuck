//

use std::thread;
use std::time::Duration;

extern crate shattuck;

use shattuck::core::error::Result;
use shattuck::core::memory::{Address, Memory};
use shattuck::core::object::{GetHoldee, Object, ToSync};
use shattuck::core::runtime::RuntimeBuilder;

extern crate rand;
use rand::random;

#[derive(Debug)]
struct Int(i32);

unsafe impl GetHoldee for Int {
    fn get_holdee(&self) -> Vec<Address> {
        Vec::new()
    }
}

impl ToSync for Int {
    type Target = Int;

    fn to_sync(self) -> Result<Self::Target> {
        Ok(self)
    }
}

fn main() {
    let mut memory = Memory::new(16);
    let context = memory.insert_local(Object::new(Int(42))).unwrap();
    let runtime = RuntimeBuilder::new(memory, context).boot().unwrap();
    // read a object
    println!(
        "{:?}",
        runtime
            .context()
            .get_ref()
            .unwrap()
            .as_ref::<Int>()
            .unwrap()
    );
    let shared = runtime.context().share::<Int>().unwrap();
    let handle = thread::spawn(move || {
        let mut memory = Memory::new(16);
        let context = memory.insert_shared(shared).unwrap();
        let runtime = RuntimeBuilder::new(memory, context).boot().unwrap();
        println!(
            "{:?}",
            runtime
                .context()
                .get_ref()
                .unwrap()
                .as_ref::<Int>()
                .unwrap()
        );
        thread::sleep(Duration::from_millis(random::<u64>() % 1000));
        *runtime
            .context()
            .get_mut()
            .unwrap()
            .as_mut::<Int>()
            .unwrap() = Int(43);
    });
    thread::sleep(Duration::from_millis(random::<u64>() % 1000));
    println!(
        "42 or 43 or panic? {:?}",
        runtime
            .context()
            .get_ref()
            .unwrap()
            .as_ref::<Int>()
            .unwrap()
    );
    handle.join().unwrap();
}
