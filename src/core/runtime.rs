//

use std::error::Error;
use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::core::memory::{Addr, Memory, MemoryError};
use crate::core::object::Object;

pub struct Runtime {
    mem: Memory,
    frame_stack: Vec<Frame>,
    context_object: Name,
}

#[derive(Debug)]
pub enum RuntimeError {
    OutOfMemory,
    UndefinedName(String),
    EmptyEnvStack,
    EmptyFrameStack,
    TypeMismatch { expected: TypeId, actual: TypeId },
    MissingObject(Name),
    NotCallable(Name),
    Unhandled(Name),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            RuntimeError::OutOfMemory => write!(f, "out of memory"),
            RuntimeError::UndefinedName(name) => write!(f, "undefined name '{}'", name),
            RuntimeError::EmptyEnvStack => write!(f, "poping last layer of env"),
            RuntimeError::EmptyFrameStack => write!(f, "poping last layter of frame"),
            RuntimeError::TypeMismatch { expected, actual } => {
                write!(f, "expected type {:?}, found {:?}", expected, actual)
            }
            RuntimeError::MissingObject(name) => write!(f, "missing object for name '{:?}'", name),
            RuntimeError::NotCallable(name) => {
                write!(f, "attempt to call non-callable object '{:?}'", name)
            }
            RuntimeError::Unhandled(name) => write!(f, "unhandled error {:?}", name),
        }
    }
}

impl Error for RuntimeError {}

fn append_to(mem: &mut Memory, object: Box<dyn Object>) -> Result<Addr, RuntimeError> {
    match mem.append_object(object) {
        Ok(addr) => Ok(addr),
        Err(mem_err) => {
            if let MemoryError::Full = mem_err {
                Err(RuntimeError::OutOfMemory)
            } else {
                panic!("expected MemoryError::Full, actual: {}", mem_err)
            }
        }
    }
}

fn get_object(mem: &Memory, name: Name) -> Result<&dyn Object, RuntimeError> {
    match mem.get_object(name.addr()) {
        Ok(object) => Ok(object),
        Err(mem_err) => {
            if let MemoryError::InvalidAddr(addr) = mem_err {
                assert_eq!(addr, name.addr());
                Err(RuntimeError::MissingObject(name))
            } else {
                panic!("expected MemoryError::InvalidAddr, actual: {}", mem_err)
            }
        }
    }
}

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

    fn find_object(&self, name: &str) -> Option<Addr> {
        self.name_map.get(name).cloned()
    }

    fn insert_object(&mut self, name: &str, object: Addr) {
        self.name_map.insert(name.to_string(), object);
    }
}

impl Object for Env {
    fn get_property(&self, key: &str) -> Option<Name> {
        Some(Name::with_addr(self.find_object(key)?))
    }

    fn set_property(&mut self, key: &str, new_prop: Name) {
        self.insert_object(key, new_prop.addr());
    }
}

impl Frame {
    fn new(mem: &mut Memory, parent: Option<Addr>) -> Result<Self, RuntimeError> {
        let first_env = append_to(mem, Box::new(Env::new()))?;
        let frame = Frame {
            env_stack: vec![first_env],
        };
        if let Some(parent_addr) = parent {
            mem.hold(first_env, parent_addr)
                .expect("first_env -> parent_addr");
        }
        mem.set_root(first_env).expect("root <- first_env");
        Ok(frame)
    }

    fn push_env(&mut self, mem: &mut Memory) -> Result<(), RuntimeError> {
        let env = append_to(mem, Box::new(Env::new()))?;
        mem.hold(env, *self.env_stack.last().expect("env_stack.last()"))
            .expect("env -> prev env");
        mem.set_root(env).expect("root <- env");
        self.env_stack.push(env);
        Ok(())
    }

    fn pop_env(&mut self, mem: &mut Memory) -> Result<(), RuntimeError> {
        self.env_stack.pop();
        mem.set_root(*self.env_stack.last().ok_or(RuntimeError::EmptyEnvStack)?)
            .expect("root <- prev env");
        Ok(())
    }

    fn insert_object(&self, mem: &mut Memory, name: &str, object: Addr) -> Result<(), MemoryError> {
        let result = mem.set_object_property(self.current_env(), name, object);
        if let Ok(_) = result {
            return Ok(());
        } else {
            if let Err(MemoryError::InvalidAddr(addr)) = result {
                assert_eq!(addr, object);
            } else {
                panic!("expected MemoryError::InvalidAddr, get {:?}", result)
            }
        }
        result
    }

    fn find_object(&self, mem: &Memory, name: &str) -> Result<Name, RuntimeError> {
        for env in self.env_stack.iter().rev() {
            if let Some(object) = mem
                .get_object(*env)
                .expect("env in env_stack")
                .get_property(name)
            {
                return Ok(object);
            }
        }
        Err(RuntimeError::UndefinedName(name.to_string()))
    }

    fn current_env(&self) -> Addr {
        self.env_stack.last().expect("current env").to_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Name(Addr);

impl Name {
    pub(crate) fn with_addr(addr: Addr) -> Self {
        Name(addr)
    }

    pub(crate) fn addr(self) -> Addr {
        self.0
    }
}

impl Runtime {
    pub fn new(max_object_count: usize) -> Result<Self, RuntimeError> {
        let mut mem = Memory::with_max_object_count(max_object_count);
        let first_frame = Frame::new(&mut mem, None)?;
        Ok(Runtime {
            mem,
            context_object: Name::with_addr(first_frame.current_env()),
            frame_stack: vec![first_frame],
        })
    }

    pub fn push_frame(&mut self) -> Result<(), RuntimeError> {
        let frame = Frame::new(
            &mut self.mem,
            self.frame_stack.last().map(|frame| frame.current_env()),
        )?;
        self.frame_stack.push(frame);
        Ok(())
    }

    pub fn pop_frame(&mut self) -> Result<(), RuntimeError> {
        self.frame_stack.pop();
        self.mem
            .set_root(
                self.frame_stack
                    .last()
                    .ok_or(RuntimeError::EmptyFrameStack)?
                    .current_env(),
            )
            .expect("root <- current env");
        Ok(())
    }

    pub fn push_env(&mut self) -> Result<(), RuntimeError> {
        let frame = self.frame_stack.last_mut().expect("current frame");
        frame.push_env(&mut self.mem)
    }

    pub fn pop_env(&mut self) -> Result<(), RuntimeError> {
        self.frame_stack
            .last_mut()
            .expect("current frame")
            .pop_env(&mut self.mem)
    }

    pub fn get_object<T: 'static>(&self, name: Name) -> Result<&T, RuntimeError> {
        let obj = get_object(&self.mem, name)?;
        obj.as_any()
            .downcast_ref::<T>()
            .ok_or(RuntimeError::TypeMismatch {
                expected: TypeId::of::<T>(),
                actual: obj.as_any().type_id(),
            })
    }

    pub fn garbage_collect(&mut self) {
        self.mem.collect();
    }

    // <name> = <object>
    pub fn append_object(&mut self, object: Box<dyn Object>) -> Result<Name, RuntimeError> {
        Ok(Name::with_addr(append_to(&mut self.mem, object)?))
    }

    // env_name = <name>
    pub fn insert_name(&mut self, name: Name, env_name: &str) -> Result<(), RuntimeError> {
        let frame = self.frame_stack.last().expect("current frame");
        frame
            .insert_object(&mut self.mem, env_name, name.addr())
            .or(Err(RuntimeError::MissingObject(name)))
    }

    // <name> = env_name
    pub fn find_name(&self, env_name: &str) -> Result<Name, RuntimeError> {
        self.frame_stack
            .last()
            .expect("current frame")
            .find_object(&self.mem, env_name)
    }

    // <name> = object.prop
    pub fn get_property(&self, object: Name, prop: &str) -> Result<Option<Name>, RuntimeError> {
        Ok(get_object(&self.mem, object)?.get_property(prop))
    }

    // object.prop = <name>
    pub fn set_property(
        &mut self,
        object: Name,
        prop: &str,
        name: Name,
    ) -> Result<(), RuntimeError> {
        let result = self
            .mem
            .set_object_property(object.addr(), prop, name.addr());
        if let Ok(_) = result {
            Ok(())
        } else {
            if let Err(MemoryError::InvalidAddr(addr)) = result {
                if addr == object.addr() {
                    Err(RuntimeError::MissingObject(object))
                } else if addr == name.addr() {
                    Err(RuntimeError::MissingObject(name))
                } else {
                    panic!("addr != object && addr != name")
                }
            } else {
                panic!("expected MemoryError::InvalidAddr, get {:?}", result)
            }
        }
    }

    // <name> = this
    pub fn context(&self) -> Name {
        self.context_object
    }

    // this = <name>
    pub fn set_context(&mut self, name: Name) {
        self.context_object = name;
    }

    // <method>(&{args})
    pub fn run_method(&mut self, method: Name) -> Result<(), RuntimeError> {
        let method_object = get_object(&self.mem, method)?
            .as_method()
            .ok_or(RuntimeError::NotCallable(method))?;

        self.push_frame()?;
        method_object.run(self)?;
        self.pop_frame()?;
        Ok(())
    }
}
