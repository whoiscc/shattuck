//

use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;

extern crate crossbeam;
use crossbeam::sync::{ShardedLock, ShardedLockReadGuard as ReadLock};

use crate::core::memory::{Addr, Memory as RawMemory, MemoryError};
use crate::core::object::{AsMethod, CloneObject, Object};
use crate::objects::prop::PropObject;

type Memory = Arc<ShardedLock<RawMemory>>;

pub struct Runtime {
    mem: Memory,
    frame_stack: Vec<Frame>,
    context_object: Addr,
}

#[derive(Debug)]
pub enum RuntimeError {
    OutOfMemory,
    UndefinedName(String),
    EmptyEnvStack,
    EmptyFrameStack,
    TypeMismatch { expected: TypeId, actual: TypeId },
    MissingObject(Addr),
    NotCallable(Addr),
    Unhandled(Addr),
    NoSuchProp(Addr, String),
}

impl From<MemoryError> for RuntimeError {
    fn from(mem_err: MemoryError) -> Self {
        match mem_err {
            MemoryError::Full => RuntimeError::OutOfMemory,
            MemoryError::InvalidAddr(addr) => RuntimeError::MissingObject(addr),
        }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            RuntimeError::OutOfMemory => write!(f, "out of memory"),
            RuntimeError::UndefinedName(pointer) => write!(f, "undefined pointer '{}'", pointer),
            RuntimeError::EmptyEnvStack => write!(f, "poping last layer of env"),
            RuntimeError::EmptyFrameStack => write!(f, "poping last layter of frame"),
            RuntimeError::TypeMismatch { expected, actual } => {
                write!(f, "expected type {:?}, found {:?}", expected, actual)
            }
            RuntimeError::MissingObject(pointer) => {
                write!(f, "missing object for pointer '{:?}'", pointer)
            }
            RuntimeError::NotCallable(pointer) => {
                write!(f, "attempt to call non-callable object '{:?}'", pointer)
            }
            RuntimeError::Unhandled(pointer) => write!(f, "unhandled error {:?}", pointer),
            RuntimeError::NoSuchProp(pointer, prop_key) => {
                write!(f, "object '{:?}' don't have property {}", pointer, prop_key)
            }
        }
    }
}

impl Error for RuntimeError {}

struct Frame {
    env_stack: Vec<Addr>,
}

struct Env {
    name_map: HashMap<String, Addr>,
}

impl Env {
    fn new() -> Self {
        Self {
            name_map: HashMap::new(),
        }
    }

    fn find_object(&self, pointer: &str) -> Option<Addr> {
        self.name_map.get(pointer).cloned()
    }

    fn insert_object(&mut self, pointer: &str, object: Addr) {
        self.name_map.insert(pointer.to_string(), object);
    }
}

impl Object for Env {}

impl PropObject for Env {
    fn get_prop(&self, key: &str) -> Option<Addr> {
        self.find_object(key)
    }

    fn set_prop(&mut self, key: &str, prop: Addr) {
        self.insert_object(key, prop)
    }
}

impl AsMethod for Env {}

impl CloneObject for Env {}

impl Frame {
    fn new(mem: Memory, parent: Option<Addr>) -> Result<Self, RuntimeError> {
        let first_env = mem.write().unwrap().append_object(Box::new(Env::new()))?;
        let frame = Frame {
            env_stack: vec![first_env],
        };
        if let Some(parent_addr) = parent {
            mem.write()
                .unwrap()
                .hold(first_env, parent_addr)
                .expect("first_env -> parent_addr");
        }
        mem.write()
            .unwrap()
            .set_root(first_env)
            .expect("root <- first_env");
        Ok(frame)
    }

    fn push_env(&mut self, mem: Memory) -> Result<(), RuntimeError> {
        let env = mem.write().unwrap().append_object(Box::new(Env::new()))?;
        mem.write()
            .unwrap()
            .hold(env, *self.env_stack.last().expect("env_stack.last()"))
            .expect("env -> prev env");
        mem.write().unwrap().set_root(env).expect("root <- env");
        self.env_stack.push(env);
        Ok(())
    }

    fn pop_env(&mut self, mem: Memory) -> Result<(), RuntimeError> {
        self.env_stack.pop();
        mem.write()
            .unwrap()
            .set_root(*self.env_stack.last().ok_or(RuntimeError::EmptyEnvStack)?)
            .expect("root <- prev env");
        Ok(())
    }

    fn insert_object(&self, mem: Memory, pointer: &str, object: Addr) -> Result<(), MemoryError> {
        mem.write()
            .unwrap()
            .set_object_property(self.current_env(), pointer, object)
    }

    fn find_object(&self, mem: Memory, pointer: &str) -> Result<Addr, RuntimeError> {
        for env in self.env_stack.iter().rev() {
            if let Some(object) = mem
                .read()
                .unwrap()
                .get_object(*env)
                .expect("env in env_stack")
                .as_prop()
                .expect("env AsProp")
                .get_prop(pointer)
            {
                return Ok(object);
            }
        }
        Err(RuntimeError::UndefinedName(pointer.to_string()))
    }

    fn current_env(&self) -> Addr {
        self.env_stack.last().expect("current env").to_owned()
    }
}

pub struct GetObject<'a>(ReadLock<'a, RawMemory>, Addr);

impl<'a> GetObject<'a> {
    pub fn to<T: Any>(&self) -> Result<&T, RuntimeError> {
        let GetObject(mem, addr) = self;
        let obj = mem.get_object(*addr)?.as_any();
        obj.downcast_ref::<T>().ok_or(RuntimeError::TypeMismatch {
            expected: TypeId::of::<T>(),
            actual: obj.type_id(),
        })
    }
}

pub fn make_shared(raw_mem: RawMemory) -> Memory {
    Arc::new(ShardedLock::new(raw_mem))
}

impl Runtime {
    pub fn new(mem: Memory) -> Result<Self, RuntimeError> {
        let first_frame = Frame::new(Arc::clone(&mem), None)?;
        Ok(Runtime {
            mem,
            context_object: first_frame.current_env(),
            frame_stack: vec![first_frame],
        })
    }

    pub fn push_frame(&mut self) -> Result<(), RuntimeError> {
        let frame = Frame::new(
            Arc::clone(&self.mem),
            self.frame_stack.last().map(Frame::current_env),
        )?;
        self.frame_stack.push(frame);
        Ok(())
    }

    pub fn pop_frame(&mut self) -> Result<(), RuntimeError> {
        let holder_env = self.frame_stack.last().unwrap().current_env();
        self.frame_stack.pop();
        let holdee_env = self
            .frame_stack
            .last()
            .ok_or(RuntimeError::EmptyFrameStack)?
            .current_env();
        // if holder_env keeps alive because returned closure, it should not cause holdee_env
        // to be alive because holdee_env is invisible to the closure
        RawMemory::drop(&mut self.mem.write().unwrap(), holder_env, holdee_env)?;
        self.mem
            .write()
            .unwrap()
            .set_root(holdee_env)
            .expect("root <- current env");
        Ok(())
    }

    pub fn push_env(&mut self) -> Result<(), RuntimeError> {
        let frame = self.frame_stack.last_mut().expect("current frame");
        frame.push_env(Arc::clone(&self.mem))
    }

    pub fn pop_env(&mut self) -> Result<(), RuntimeError> {
        self.frame_stack
            .last_mut()
            .expect("current frame")
            .pop_env(Arc::clone(&self.mem))
    }

    pub fn get_object(&self, addr: Addr) -> GetObject {
        GetObject(self.mem.read().unwrap(), addr)
    }

    pub fn garbage_collect(&mut self) {
        self.mem.write().unwrap().collect();
    }

    // <pointer> = <object>
    pub fn append_object(&mut self, object: Box<dyn Object>) -> Result<Addr, RuntimeError> {
        self.mem
            .write()
            .unwrap()
            .append_object(object)
            .map_err(Into::into)
    }

    // env_name = <pointer>
    pub fn insert_name(&mut self, name: &str, addr: Addr) -> Result<(), RuntimeError> {
        let frame = self.frame_stack.last().expect("current frame");
        frame
            .insert_object(Arc::clone(&self.mem), name, addr)
            .map_err(Into::into)
    }

    // <pointer> = env_name
    pub fn find_name(&self, env_name: &str) -> Result<Addr, RuntimeError> {
        self.frame_stack
            .last()
            .expect("current frame")
            .find_object(Arc::clone(&self.mem), env_name)
    }

    // <pointer> = object.prop
    pub fn get_property(&self, object: Addr, prop: &str) -> Result<Addr, RuntimeError> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .get_object(object)?
            .as_prop()
            .unwrap() // TODO
            .get_prop(prop)
            .ok_or_else(|| RuntimeError::NoSuchProp(object, prop.to_string()))?)
    }

    // object.prop = <pointer>
    pub fn set_property(
        &mut self,
        object: Addr,
        prop: &str,
        addr: Addr,
    ) -> Result<(), RuntimeError> {
        self.mem
            .write()
            .unwrap()
            .set_object_property(object, prop, addr)
            .map_err(Into::into)
    }

    // <pointer> = this
    pub fn context(&self) -> Addr {
        self.context_object
    }

    // this = <pointer>
    pub fn set_context(&mut self, addr: Addr) {
        self.context_object = addr;
    }

    // <method>(&{args})
    pub fn run_method(&mut self, method: Addr) -> Result<(), RuntimeError> {
        self.push_frame()?;

        let cloned_method = self
            .mem
            .read()
            .unwrap()
            .get_object(method)?
            .clone_object()
            .unwrap(); // TODO
        let method_object = cloned_method
            .as_method()
            .ok_or_else(|| RuntimeError::NotCallable(method))?;
        method_object.run(self)?;

        self.pop_frame()?;
        Ok(())
    }
}
