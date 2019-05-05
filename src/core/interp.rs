//

use std::any::TypeId;
use std::collections::HashMap;

use crate::core::memory::{Addr, Memory, MemoryError};
use crate::core::object::Object;

pub struct Interp {
    mem: Memory,
    frame_stack: Vec<Frame>,
    context_object: Name,
}

#[derive(Debug)]
pub enum InterpError {
    OutOfMemory,
    UndefinedName(String),
    EmptyEnvStack,
    EmptyFrameStack,
    TypeMismatch { expected: TypeId, actual: TypeId },
    MissingObject(Name),
    NotCallable(Name),
}

fn append_to(mem: &mut Memory, object: Box<dyn Object>) -> Result<Addr, InterpError> {
    match mem.append_object(object) {
        Ok(addr) => Ok(addr),
        Err(mem_err) => {
            if let MemoryError::Full = mem_err {
                Err(InterpError::OutOfMemory)
            } else {
                panic!("expected MemoryError::Full, actual: {}", mem_err)
            }
        }
    }
}

fn get_object(mem: &Memory, name: Name) -> Result<&dyn Object, InterpError> {
    match mem.get_object(name.addr()) {
        Ok(object) => Ok(object),
        Err(mem_err) => {
            if let MemoryError::InvalidAddr(addr) = mem_err {
                assert_eq!(addr, name.addr());
                Err(InterpError::MissingObject(name))
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
    fn new(mem: &mut Memory, parent: Option<Addr>) -> Result<Self, InterpError> {
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

    fn push_env(&mut self, mem: &mut Memory) -> Result<(), InterpError> {
        let env = append_to(mem, Box::new(Env::new()))?;
        mem.hold(env, *self.env_stack.last().expect("env_stack.last()"))
            .expect("env -> prev env");
        mem.set_root(env).expect("root <- env");
        self.env_stack.push(env);
        Ok(())
    }

    fn pop_env(&mut self, mem: &mut Memory) -> Result<(), InterpError> {
        self.env_stack.pop();
        mem.set_root(*self.env_stack.last().ok_or(InterpError::EmptyEnvStack)?)
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

    fn find_object(&self, mem: &Memory, name: &str) -> Result<Name, InterpError> {
        for env in self.env_stack.iter().rev() {
            if let Some(object) = mem
                .get_object(*env)
                .expect("env in env_stack")
                .get_property(name)
            {
                return Ok(object);
            }
        }
        Err(InterpError::UndefinedName(name.to_string()))
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

impl Interp {
    pub fn new(max_object_count: usize) -> Result<Self, InterpError> {
        let mut mem = Memory::with_max_object_count(max_object_count);
        let first_frame = Frame::new(&mut mem, None)?;
        Ok(Interp {
            mem,
            context_object: Name::with_addr(first_frame.current_env()),
            frame_stack: vec![first_frame],
        })
    }

    pub fn push_frame(&mut self) -> Result<(), InterpError> {
        let frame = Frame::new(
            &mut self.mem,
            self.frame_stack.last().map(|frame| frame.current_env()),
        )?;
        self.frame_stack.push(frame);
        Ok(())
    }

    pub fn pop_frame(&mut self) -> Result<(), InterpError> {
        self.frame_stack.pop();
        self.mem.set_root(
            self.frame_stack
                .last()
                .ok_or(InterpError::EmptyFrameStack)?
                .current_env(),
        ).expect("root <- current env");
        Ok(())
    }

    pub fn push_env(&mut self) -> Result<(), InterpError> {
        let frame = self.frame_stack.last_mut().expect("current frame");
        frame.push_env(&mut self.mem)
    }

    pub fn pop_env(&mut self) -> Result<(), InterpError> {
        self.frame_stack
            .last_mut()
            .expect("current frame")
            .pop_env(&mut self.mem)
    }

    pub fn get_object<T: 'static>(&self, name: Name) -> Result<&T, InterpError> {
        let obj = get_object(&self.mem, name)?;
        obj.as_any()
            .downcast_ref::<T>()
            .ok_or(InterpError::TypeMismatch {
                expected: TypeId::of::<T>(),
                actual: obj.as_any().type_id(),
            })
    }

    pub fn garbage_collect(&mut self) {
        self.mem.collect();
    }

    // <name> = <object>
    pub fn append_object(&mut self, object: Box<dyn Object>) -> Result<Name, InterpError> {
        Ok(Name::with_addr(append_to(&mut self.mem, object)?))
    }

    // env_name = <name>
    pub fn insert_name(&mut self, name: Name, env_name: &str) -> Result<(), InterpError> {
        let frame = self.frame_stack.last().expect("current frame");
        frame
            .insert_object(&mut self.mem, env_name, name.addr())
            .or(Err(InterpError::MissingObject(name)))
    }

    // <name> = env_name
    pub fn find_name(&self, env_name: &str) -> Result<Name, InterpError> {
        self.frame_stack
            .last()
            .expect("current frame")
            .find_object(&self.mem, env_name)
    }

    // <name> = object.prop
    pub fn get_property(&self, object: Name, prop: &str) -> Result<Option<Name>, InterpError> {
        Ok(get_object(&self.mem, object)?.get_property(prop))
    }

    // object.prop = <name>
    pub fn set_property(
        &mut self,
        object: Name,
        prop: &str,
        name: Name,
    ) -> Result<(), InterpError> {
        let result = self
            .mem
            .set_object_property(object.addr(), prop, name.addr());
        if let Ok(_) = result {
            Ok(())
        } else {
            if let Err(MemoryError::InvalidAddr(addr)) = result {
                if addr == object.addr() {
                    Err(InterpError::MissingObject(object))
                } else if addr == name.addr() {
                    Err(InterpError::MissingObject(name))
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
    pub fn run_method(&mut self, method: Name) -> Result<(), InterpError> {
        let method_object = get_object(&self.mem, method)?
            .as_method()
            .ok_or(InterpError::NotCallable(method))?;

        self.push_frame()?;
        method_object.run(self)?;
        self.pop_frame()?;
        Ok(())
    }
}
