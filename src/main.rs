//

use std::ops::{Deref, DerefMut};
use std::thread;

extern crate shattuck;
use shattuck::core::runtime::Runtime;

struct SharedNum(i64);

impl From<Box<i64>> for SharedNum {
    fn from(n: Box<i64>) -> SharedNum {
        SharedNum(*n)
    }
}

impl Deref for SharedNum {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedNum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn main() {
    let mut host_runtime = Runtime::<Box<i64>, SharedNum>::new(16);
    let host_id = host_runtime.insert(Box::new(42)).unwrap();
    let share = host_runtime.share(host_id).unwrap();
    let handle = thread::spawn(|| {
        let mut guest_runtime = Runtime::<Box<i64>, SharedNum>::new(16);
        let guest_id = guest_runtime.insert_remote(share).unwrap();
        {
            let guest_object = guest_runtime.read(guest_id).unwrap();
            println!("{:?}", &guest_object as &i64);
        }
        *(&mut guest_runtime.write(guest_id).unwrap() as &mut i64) = 43;
    });
    handle.join().unwrap();
    println!("{:?}", &host_runtime.read(host_id).unwrap() as &i64);
}
