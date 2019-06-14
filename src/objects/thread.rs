//

use std::thread::{spawn, JoinHandle};

use crate::core::error::Result;
use crate::core::memory::{Address, Memory};
use crate::core::object::{GetHoldee, Object, SyncObject, ToSync};
use crate::core::runtime::{Method, RuntimeBuilder};

pub fn make_thread(method: Address) -> Method {
    Method::new(
        |runtime| {
            let sync_method = runtime.context().share()?;
            let mut args: Vec<SyncObject> = Vec::new();
            while let Ok(mut address) = runtime.get(1) {
                runtime.pop()?;
                args.push(address.share()?);
            }
            let handle: JoinHandle<Result<_>> = spawn(move || {
                let mut memory = Memory::new(1024);
                let method = memory.insert_shared(sync_method)?;
                let mut runtime = RuntimeBuilder::new(memory, method).boot()?;
                let arg_count = args.len();
                // `runtime` stack keeps arguments in reverse order
                for arg in args.into_iter().rev() {
                    let shared_arg = runtime.memory.insert_shared(arg)?;
                    runtime.push(shared_arg);
                }
                let result_count = runtime.call(method, &(1..=arg_count).collect::<Vec<_>>())?;
                let mut shared_results: Vec<SyncObject> = Vec::new();
                for _ in 0..result_count {
                    shared_results.push(runtime.get(1)?.share()?);
                    runtime.pop()?;
                }
                Ok(shared_results)
            });
            let join = runtime
                .memory
                .insert_local(Object::new(Join(Some(handle))))?;
            let join_method = runtime.memory.insert_local(Object::new(make_join(join)))?;
            runtime.push(join_method);
            runtime.push_parent(1)?;
            Ok(())
        },
        method,
    )
}

type JoinT = JoinHandle<Result<Vec<SyncObject>>>;

struct Join(Option<JoinT>);

unsafe impl GetHoldee for Join {
    fn get_holdee(&self) -> Vec<Address> {
        Vec::new()
    }
}

impl ToSync for Join {
    type Target = Join;

    fn to_sync(self) -> Result<Self::Target> {
        Ok(self)
    }
}

fn make_join(join: Address) -> Method {
    Method::new(
        |runtime| {
            let mut get_context = runtime.context();
            let mut get_mut_option_join = get_context.get_mut()?;
            let option_handle = &mut get_mut_option_join.as_mut::<Join>()?.0;
            if option_handle.is_none() {
                unimplemented!();
            }
            let handle = option_handle.take().unwrap();
            let results = handle.join().unwrap()?;
            for result_object in results.into_iter().rev() {
                let result = runtime.memory.insert_shared(result_object)?;
                runtime.push(result);
                runtime.push_parent(1)?;
            }
            Ok(())
        },
        join,
    )
}
