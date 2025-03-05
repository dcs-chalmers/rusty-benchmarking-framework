#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::{ConcurrentQueue, Handle};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// A safe Rust wrapper around the C bindings
pub struct MoodyCamelCppQueue {
    raw: MoodyCamelConcurrentQueue,
}

unsafe impl Send for MoodyCamelCppQueue {}
unsafe impl Sync for MoodyCamelCppQueue {}

impl MoodyCamelCppQueue {
    
    pub fn push(&self, item: *mut std::ffi::c_void) -> bool {
        unsafe { moody_camel_push(self.raw, item) == 1 }
    }
    
    pub fn pop(&self) -> Option<*mut std::ffi::c_void> {
        let mut item: *mut std::ffi::c_void = std::ptr::null_mut();
        let success = unsafe { moody_camel_pop(self.raw, &mut item) == 1 };
        if success {
            Some(item)
        } else {
            None
        }
    }
    
}

impl Drop for MoodyCamelCppQueue {
    fn drop(&mut self) {
        unsafe { moody_camel_destroy(self.raw) };
    }
}

struct MoodyCamelCppQueueHandle<'a> {
    pub q: &'a MoodyCamelCppQueue
}

impl Handle<Box<i32>> for MoodyCamelCppQueueHandle<'_> {
    fn push(&mut self, item: Box<i32>) {
        let ptr: *mut std::ffi::c_void = Box::<i32>::into_raw(item) as *mut std::ffi::c_void;
        assert!(self.q.push(ptr));
    }

    fn pop(&mut self) -> Option<Box<i32>> {
        let res = match self.q.pop() {
            Some(v) => v,
            None => return None,
        };
        let val = unsafe { Box::from_raw(res as *const i32 as *mut i32) };
        Some(val)
    }
}

impl ConcurrentQueue<Box<i32>> for MoodyCamelCppQueue {
    fn register(&self) -> impl crate::Handle<Box<i32>> {
        MoodyCamelCppQueueHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("moodycamel")
    }

    fn new(_capacity: usize) -> Self {
        let raw = unsafe { moody_camel_create() };
        MoodyCamelCppQueue { raw }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_moody_camel() {
        let q: MoodyCamelCppQueue = MoodyCamelCppQueue::new(1000);
        let _ = q.push(Box::<i32>::into_raw(Box::new(32)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(33)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(34)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(35)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(36)) as *mut std::ffi::c_void);
        let val = unsafe {*(q.pop().unwrap() as *const i32)};
        assert_eq!(val, 32);
        let val = unsafe {*(q.pop().unwrap() as *const i32)};
        assert_eq!(val, 33);
        let val = unsafe {*(q.pop().unwrap() as *const i32)};
        assert_eq!(val, 34);
        let val = unsafe {*(q.pop().unwrap() as *const i32)};
        assert_eq!(val, 35);
        let val = unsafe {*(q.pop().unwrap() as *const i32)};
        assert_eq!(val, 36);
    }
    #[test]
    fn register_moody_camel() {
        let q: MoodyCamelCppQueue = MoodyCamelCppQueue::new(1000);
        let mut handle = q.register();
        handle.push(Box::new(1));
        handle.push(Box::new(2));
        handle.push(Box::new(3));
        handle.push(Box::new(4));
        assert_eq!(*handle.pop().unwrap(), 1);
        assert_eq!(*handle.pop().unwrap(), 2);
        assert_eq!(*handle.pop().unwrap(), 3);
        assert_eq!(*handle.pop().unwrap(), 4);
    }
}
