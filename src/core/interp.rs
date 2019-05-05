//

use std::collections::HashMap;

use crate::core::memory::{Addr, Memory};
use crate::core::object::{as_type, Object};
use crate::objects::method::MethodObject;

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
    fn new() -> Self {
        Frame {
            env_stack: Vec::new(),
        }
    }

    fn push_env(&mut self, mem: &mut Memory) {
        let env = mem.append_object(Box::new(Env::new())).unwrap(); // TODO
        self.env_stack.push(env);
    }

    fn pop_env(&mut self) {
        self.env_stack.pop();
    }

    fn insert_object(&self, mem: &mut Memory, name: &str, object: Addr) {
        mem.set_object_property(self.current_env(), name, object);
        // backward holding, is it ok?
        mem.hold(object, self.current_env())
    }

    fn find_object(&self, mem: &Memory, name: &str) -> Option<Name> {
        mem.get_object(self.current_env())?.get_property(name)
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
    pub fn new(initial_self: Box<dyn Object>, max_object_count: usize) -> Self {
        let mut mem = Memory::with_max_object_count(max_object_count);
        let context_object = mem.append_object(initial_self).unwrap();
        mem.set_root(context_object);
        Interp {
            mem,
            context_object,
            frame_stack: Vec::new(),
        }
    }

    pub fn push_frame(&mut self) {
        self.frame_stack.push(Frame::new());
    }

    pub fn pop_frame(&mut self) {
        self.frame_stack.pop();
    }

    pub fn push_env(&mut self) {
        let frame = self.frame_stack.last_mut().unwrap();
        frame.push_env(&mut self.mem);
    }

    pub fn pop_env(&mut self) {
        self.frame_stack.last_mut().unwrap().pop_env();
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
        let method_object: &MethodObject = self.get_object(method)?;
        let internal_method = method_object.run.to_owned();

        let backup_context = self.context();
        self.set_context(self.get_property(method, "context").unwrap());
        self.push_frame();
        self.push_env();
        internal_method(self);
        self.pop_env();
        self.pop_frame();
        self.set_context(backup_context);
        Some(())
    }
}
