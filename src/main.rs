//

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::thread;

extern crate shattuck;
use shattuck::core::runtime::Runtime;

trait DuckNum {
    fn get_num(&self) -> i64;
    fn set_num(&mut self, n: i64);
}

struct SharedNum(i64);

impl DuckNum for SharedNum {
    fn get_num(&self) -> i64 {
        self.0
    }

    fn set_num(&mut self, n: i64) {
        self.0 = n;
    }
}

struct LocalNum(RefCell<i64>);

impl DuckNum for LocalNum {
    fn get_num(&self) -> i64 {
        *self.0.borrow()
    }

    fn set_num(&mut self, n: i64) {
        *self.0.borrow_mut() = n;
    }
}

impl From<LocalNum> for SharedNum {
    fn from(local: LocalNum) -> SharedNum {
        SharedNum(*local.0.borrow())
    }
}

impl Deref for SharedNum {
    type Target = dyn DuckNum;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for SharedNum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

impl Deref for LocalNum {
    type Target = dyn DuckNum;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for LocalNum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

fn main() {
    let mut host_runtime = Runtime::<LocalNum, SharedNum>::new(16);
    let host_id = host_runtime.insert(LocalNum(RefCell::new(42))).unwrap();
    let share = host_runtime.share(host_id).unwrap();
    let handle = thread::spawn(|| {
        let mut guest_runtime = Runtime::<LocalNum, SharedNum>::new(16);
        let guest_id = guest_runtime.insert_remote(share).unwrap();
        {
            let guest_object = guest_runtime.read(guest_id).unwrap();
            println!("{:?}", guest_object.get_num());
        }
        guest_runtime.write(guest_id).unwrap().set_num(43);
    });
    handle.join().unwrap();
    println!("{:?}", host_runtime.read(host_id).unwrap().get_num());
}
