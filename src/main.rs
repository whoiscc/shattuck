//

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::thread;

extern crate shattuck;
use shattuck::core::runtime::{AsMethod, Method, Runtime};
use shattuck::core::runtime_error::RuntimeError;
use shattuck::util::addr_gen::Inc;

trait DuckNum {
    fn get_num(&self) -> i64;
    fn set_num(&mut self, n: i64);

    fn println(&self) {
        println!("DuckNum: {}", self.get_num());
    }
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

type R = Runtime<LocalNum, SharedNum, usize, Inc>;

impl Method<LocalNum, SharedNum, usize, Inc> for SharedNum {
    fn run(&self, _runtime: &mut R) -> Result<(), RuntimeError> {
        self.println();
        Ok(())
    }
}

impl AsMethod<LocalNum, SharedNum, usize, Inc> for SharedNum {
    fn as_method(&self) -> Result<&dyn Method<LocalNum, SharedNum, usize, Inc>, RuntimeError> {
        Ok(self)
    }
}

fn main() {
    let mut host_runtime = R::new(16, Inc::new());
    let host_id = host_runtime.insert(LocalNum(RefCell::new(42))).unwrap();
    let share = host_runtime.share(&host_id).unwrap();
    let handle = thread::spawn(|| {
        let mut guest_runtime = R::new(16, Inc::new());
        let guest_id = guest_runtime.insert_remote(share).unwrap();
        guest_runtime.call(&guest_id).unwrap();
        guest_runtime.write(&guest_id).unwrap().set_num(43);
    });
    handle.join().unwrap();
    host_runtime.call(&host_id).unwrap();
}
