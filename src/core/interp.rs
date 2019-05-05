//

use std::collections::HashMap;

use crate::core::memory::{Addr, Memory};
use crate::core::object::{as_type, Object};

pub struct Interp {
    mem: Memory,
    frame_stack: Vec<Frame>,
    context_object: Addr,
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
    fn new(mem: &mut Memory, parent: Option<Addr>) -> Self {
        let first_env = mem.append_object(Box::new(Env::new())).unwrap();
        let frame = Frame {
            env_stack: vec![first_env],
        };
        if let Some(parent_addr) = parent {
            mem.hold(first_env, parent_addr);
        }
        mem.set_root(first_env);
        frame
    }

    fn push_env(&mut self, mem: &mut Memory) -> Addr {
        let env = mem.append_object(Box::new(Env::new())).unwrap();
        mem.hold(env, *self.env_stack.last().unwrap());
        mem.set_root(env);
        self.env_stack.push(env);
        env
    }

    fn pop_env(&mut self, mem: &mut Memory) {
        self.env_stack.pop();
        mem.set_root(*self.env_stack.last().unwrap());
    }

    fn insert_object(&self, mem: &mut Memory, name: &str, object: Addr) {
        mem.set_object_property(self.current_env(), name, object);
    }

    fn find_object(&self, mem: &Memory, name: &str) -> Option<Name> {
        for env in self.env_stack.iter().rev() {
            if let Some(object) = mem.get_object(*env)?.get_property(name) {
                return Some(object);
            }
        }
        None
    }

    fn current_env(&self) -> Addr {
        self.env_stack.last().unwrap().to_owned()
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
    pub fn new(max_object_count: usize) -> Self {
        let mut mem = Memory::with_max_object_count(max_object_count);
        let first_frame = Frame::new(&mut mem, None);
        Interp {
            mem,
            context_object: first_frame.current_env(),
            frame_stack: vec![first_frame],
        }
    }

    pub fn push_frame(&mut self) {
        let frame = Frame::new(
            &mut self.mem,
            self.frame_stack.last().map(|frame| frame.current_env()),
        );
        self.frame_stack.push(frame);
    }

    pub fn pop_frame(&mut self) {
        self.frame_stack.pop();
        self.mem.set_root(self.frame_stack.last().unwrap().current_env());
    }

    pub fn push_env(&mut self) {
        let frame = self.frame_stack.last_mut().unwrap();
        frame.push_env(&mut self.mem);
    }

    pub fn pop_env(&mut self) {
        self.frame_stack.last_mut().unwrap().pop_env(&mut self.mem);
    }

    fn get_object_by_addr<T: 'static>(&self, addr: Addr) -> Option<&T> {
        as_type::<T>(self.mem.get_object(addr)?)
    }

    pub fn get_object<T: 'static>(&self, name: Name) -> Option<&T> {
        self.get_object_by_addr(name.addr())
    }

    pub fn garbage_collect(&mut self) {
        self.mem.collect();
    }

    // <name> = <object>
    pub fn append_object(&mut self, object: Box<dyn Object>) -> Option<Name> {
        Some(Name(self.mem.append_object(object)?))
    }

    // env_name = <name>
    pub fn insert_name(&mut self, name: Name, env_name: &str) {
        let frame = self.frame_stack.last().unwrap();
        frame.insert_object(&mut self.mem, env_name, name.addr());
    }

    // <name> = env_name
    pub fn find_name(&self, env_name: &str) -> Option<Name> {
        self.frame_stack
            .last()
            .unwrap()
            .find_object(&self.mem, env_name)
    }

    // <name> = object.prop
    pub fn get_property(&self, object: Name, prop: &str) -> Option<Name> {
        Some(self.mem.get_object(object.addr())?.get_property(prop)?)
    }

    // object.prop = <name>
    pub fn set_property(&mut self, object: Name, prop: &str, name: Name) {
        self.mem
            .set_object_property(object.addr(), prop, name.addr());
    }

    // <name> = this
    pub fn context(&self) -> Name {
        Name(self.context_object)
    }

    // this = <name>
    pub fn set_context(&mut self, name: Name) {
        self.context_object = name.addr();
    }

    // <method>(&{args})
    pub fn run_method(&mut self, method: Name) -> Option<()> {
        let method_object = self.mem.get_object(method.addr())?.as_method()?;

        self.push_frame();
        method_object.run(self);
        self.pop_frame();
        Some(())
    }
}
