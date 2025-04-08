#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::traits::{ConcurrentQueue, Handle};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// A safe Rust wrapper around the C bindings
pub struct BoostCppQueue {
    raw: BoostLockfreeQueue,
}

unsafe impl Send for BoostCppQueue {}
unsafe impl Sync for BoostCppQueue {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl BoostCppQueue {
    
    pub fn push(&self, item: *mut std::ffi::c_void) -> bool {
        unsafe { 
            boost_queue_push(self.raw, item) == 1 
        }
    }
    
    pub fn pop(&self) -> Option<*mut std::ffi::c_void> {
        let mut item: *mut std::ffi::c_void = std::ptr::null_mut();
        let success = unsafe { boost_queue_pop(self.raw, &mut item) == 1 };
        if success {
            Some(item)
        } else {
            None
        }
    }
    
}

impl Drop for BoostCppQueue {
    fn drop(&mut self) {
        unsafe { boost_queue_destroy(self.raw) };
    }
}

struct BoostCppQueueHandle<'a> {
    pub q: &'a BoostCppQueue
}

impl Handle<Box<i32>> for BoostCppQueueHandle<'_> {
    fn push(&mut self, item: Box<i32>) -> Result<(), Box<i32>> {
        let ptr: *mut std::ffi::c_void = Box::<i32>::into_raw(item) as *mut std::ffi::c_void;
        match self.q.push(ptr) {
            true => Ok(()),
            false => {
                // Really unsure if this is possible
                let reclaimed: Box<i32> = unsafe { Box::from_raw(ptr as *mut i32) };
                Err(reclaimed)
            },
        }
    }

    fn pop(&mut self) -> Option<Box<i32>> {
        let res = self.q.pop()?;
        let val = unsafe { Box::from_raw(res as *const i32 as *mut i32) };
        Some(val)
    }
}

impl ConcurrentQueue<Box<i32>> for BoostCppQueue {
    fn register(&self) -> impl Handle<Box<i32>> {
        BoostCppQueueHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("boost")
    }

    fn new(capacity: usize) -> Self {
        let raw = unsafe { boost_queue_create(capacity as u32) };
        BoostCppQueue { raw }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_boost_queue() {
        let q: BoostCppQueue = BoostCppQueue::new(1000);
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
    fn register_boost_queue() {
        let q: BoostCppQueue = BoostCppQueue::new(1000);
        let mut handle = q.register();
        handle.push(Box::new(1)).unwrap();
        handle.push(Box::new(2)).unwrap();
        handle.push(Box::new(3)).unwrap();
        handle.push(Box::new(4)).unwrap();
        assert_eq!(*handle.pop().unwrap(), 1);
        assert_eq!(*handle.pop().unwrap(), 2);
        assert_eq!(*handle.pop().unwrap(), 3);
        assert_eq!(*handle.pop().unwrap(), 4);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: BoostCppQueue = BoostCppQueue::new(10);
        if crate::order::benchmark_order_box(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
