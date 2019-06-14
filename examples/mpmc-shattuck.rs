//

use std::collections::VecDeque;

extern crate crossbeam;
use crossbeam::sync::{Parker, Unparker};

extern crate shattuck;
use shattuck::core::error::Result;
use shattuck::core::memory::{Address, Memory};
use shattuck::core::object::{GetHoldee, Object, SyncObject, ToSync};
use shattuck::core::runtime::{Method, Runtime, RuntimeBuilder};
use shattuck::objects::thread::make_thread;

struct Queue {
    internal: VecDeque<i64>,
    max_len: usize,
    idle_workers: Vec<Unparker>,
    idle_hosts: Vec<Unparker>,
}

type RawResult<T, E> = std::result::Result<T, E>;

impl Queue {
    fn new(max_len: usize) -> Self {
        Self {
            max_len,
            internal: VecDeque::new(),
            idle_workers: Vec::new(),
            idle_hosts: Vec::new(),
        }
    }

    fn push_back(&mut self, object: i64) {
        self.internal.push_back(object);
        if let Some(idle_worker) = self.idle_workers.pop() {
            idle_worker.unpark();
        }
    }

    fn pop_front(&mut self) -> i64 {
        let object = self.internal.pop_front().unwrap();
        if let Some(idle_host) = self.idle_hosts.pop() {
            idle_host.unpark();
        }
        object
    }

    fn is_empty(&self) -> bool {
        self.internal.is_empty()
    }

    fn is_full(&self) -> bool {
        self.internal.len() == self.max_len
    }

    fn register_worker(&mut self, unparker: Unparker) {
        self.idle_workers.push(unparker);
    }

    fn register_host(&mut self, unparker: Unparker) {
        self.idle_hosts.push(unparker);
    }

    fn push_back_safe(&mut self, object: i64) -> RawResult<(), Parker> {
        if self.is_full() {
            let parker = Parker::new();
            self.register_host(parker.unparker().clone());
            Err(parker)
        } else {
            self.push_back(object);
            Ok(())
        }
    }

    fn pop_front_safe(&mut self) -> RawResult<i64, Parker> {
        if self.is_empty() {
            let parker = Parker::new();
            self.register_worker(parker.unparker().clone());
            Err(parker)
        } else {
            Ok(self.pop_front())
        }
    }
}

unsafe impl GetHoldee for Queue {
    fn get_holdee(&self) -> Vec<Address> {
        Vec::new()
    }
}

impl ToSync for Queue {
    type Target = Queue;

    fn to_sync(self) -> Result<Self::Target> {
        Ok(self)
    }
}

struct Int(i64);

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

// consumer_loop: (input) -> Int
fn consumer_loop(runtime: &mut Runtime) -> Result<()> {
    let mut input = runtime.get(1)?;
    let mut sum = 0;
    loop {
        let task = input.sync_mut().as_mut::<Queue>()?.pop_front_safe();
        if let Err(parker) = task {
            parker.park();
            continue;
        }
        let task = task.unwrap();
        if task == 0 {
            let sum = runtime.memory.insert_shared(SyncObject::new(Int(sum)))?;
            runtime.push(sum);
            runtime.push_parent(1)?;
            return Ok(());
        } else {
            sum += task;
        }
    }
}

// producer_loop: (output) -> void
fn producer_loop(runtime: &mut Runtime) -> Result<()> {
    let mut output = runtime.get(1)?;
    for _ in 0..1024 {
        for i in -100..=100 {
            if i == 0 {
                continue;
            }
            loop {
                let status = output.sync_mut().as_mut::<Queue>()?.push_back_safe(i);
                if let Err(parker) = status {
                    parker.park();
                } else {
                    break;
                }
            }
        }
    }
    loop {
        let status = output.sync_mut().as_mut::<Queue>()?.push_back_safe(0);
        if let Err(parker) = status {
            parker.park();
        } else {
            break;
        }
    }
    Ok(())
}

struct Nil;

unsafe impl GetHoldee for Nil {
    fn get_holdee(&self) -> Vec<Address> {
        Vec::new()
    }
}

impl ToSync for Nil {
    type Target = Nil;

    fn to_sync(self) -> Result<Self::Target> {
        Ok(self)
    }
}

fn main() {
    let mut memory = Memory::new(1024);
    let context = memory.insert_local(Object::new(Nil)).unwrap();
    let mut runtime = RuntimeBuilder::new(memory, context).boot().unwrap();
    let producer_method = Method::new(producer_loop, context);
    let consumer_method = Method::new(consumer_loop, context);
    let produce = runtime
        .memory
        .insert_local(Object::new(producer_method))
        .unwrap();
    let consume = runtime
        .memory
        .insert_local(Object::new(consumer_method))
        .unwrap();
    let thread_producer_method = make_thread(produce);
    let thread_consumer_method = make_thread(consume);
    let thread_produce = runtime
        .memory
        .insert_local(Object::new(thread_producer_method))
        .unwrap();
    let thread_consume = runtime
        .memory
        .insert_local(Object::new(thread_consumer_method))
        .unwrap();
    let queue = runtime
        .memory
        .insert_shared(SyncObject::new(Queue::new(10)))
        .unwrap();
    runtime.push(queue);

    for i in 0..3 {
        runtime.call(thread_consume, &[i * 2 + 1]).unwrap();
        runtime.call(thread_produce, &[i * 2 + 2]).unwrap();
    }

    let mut sum = [0; 3];
    for i in 0..3 {
        let produce_join = runtime.get(1).unwrap();
        runtime.pop().unwrap();
        runtime.call(produce_join, &[]).unwrap();
        let consume_join = runtime.get(1).unwrap();
        runtime.pop().unwrap();
        runtime.call(consume_join, &[]).unwrap();
        sum[i] = runtime
            .get(1)
            .unwrap()
            .get_ref()
            .unwrap()
            .as_ref::<Int>()
            .unwrap()
            .0;
        runtime.pop().unwrap();
    }

    println!(
        "{} + {} + {} = {}",
        sum[0],
        sum[1],
        sum[2],
        sum[0] + sum[1] + sum[2]
    );
}
