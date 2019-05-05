//

use std::collections::HashMap;

use crate::core::memory::{Addr, Memory};
use crate::core::object::Object;

pub struct Interp {
    mem: Memory,
    frames: Vec<Frame>,
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
    fn get_property(&self, key: &str) -> Option<Addr> {
        self.find_object(key)
    }

    fn set_property(&mut self, key: &str, new_prop: Addr) {
        self.insert_object(key, new_prop);
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

    fn find_object(&self, mem: &Memory, name: &str) -> Option<Addr> {
        mem.get_object_property(self.current_env(), name)
    }

    fn current_env(&self) -> Addr {
        self.env_stack.last().unwrap().to_owned()
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
            frames: Vec::new(),
        }
    }

    pub fn push_frame(&mut self) {
        self.frames.push(Frame::new());
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn push_env(&mut self) {
        let frame = self.frames.last_mut().unwrap();
        frame.push_env(&mut self.mem);
    }

    pub fn pop_env(&mut self) {
        self.frames.last_mut().unwrap().pop_env();
    }

    pub fn find_object(&self, name: &str) -> Option<Addr> {
        self.frames.last().unwrap().find_object(&self.mem, name)
    }

    pub fn insert_object(&mut self, name: &str, object: Box<dyn Object>) {
        let obj_addr = self.mem.append_object(object).unwrap();
        self.alias_object(name, obj_addr)
    }

    pub fn alias_object(&mut self, name: &str, object: Addr) {
        let frame = self.frames.last_mut().unwrap();
        frame.insert_object(&mut self.mem, name, object);
    }

    // object.key = new_prop
    pub fn set_object_property(&mut self, object: &str, key: &str, new_prop: &str) -> Option<()> {
        let obj_addr = self.find_object(object)?;
        self.set_object_property_(obj_addr, key, new_prop)
    }

    // this.key = new_prop
    pub fn set_context_object_property(&mut self, key: &str, new_prop: &str) -> Option<()> {
        self.set_object_property_(self.context_object, key, new_prop)
    }

    fn set_object_property_(&mut self, object: Addr, key: &str, new_prop: &str) -> Option<()> {
        let prop_addr = self.find_object(new_prop)?;
        self.mem.set_object_property(object, key, prop_addr);
        Some(())
    }

    // name = object.key
    pub fn get_object_property(&mut self, name: &str, object: &str, key: &str) -> Option<()> {
        let obj_addr = self.find_object(object)?;
        self.get_object_property_(name, obj_addr, key)
    }

    // name = this.key
    pub fn get_context_object_property(&mut self, name: &str, key: &str) -> Option<()> {
        self.get_object_property_(name, self.context_object, key)
    }

    fn get_object_property_(&mut self, name: &str, object: Addr, key: &str) -> Option<()> {
        let prop_addr = self.mem.get_object_property(object, key)?;
        self.alias_object(name, prop_addr);
        Some(())
    }

    pub fn get_object_property_raw(&self, object: Addr, key: &str) -> Option<Addr> {
        self.mem.get_object_property(object, key)
    }

    pub fn get_raw_object(&self, addr: Addr) -> Option<&Box<dyn Object>> {
        self.mem.get_object(addr)
    }

    pub fn change_context_object(&mut self, object: Addr) {
        self.context_object = object;
    }

    pub fn get_context_object(&self) -> Addr {
        self.context_object
    }

    pub fn garbage_collect(&mut self) {
        self.mem.collect();
    }
}
