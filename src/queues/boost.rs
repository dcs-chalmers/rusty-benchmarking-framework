#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::traits::{ConcurrentQueue, Handle};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// A safe Rust wrapper around the C bindings
pub struct BoostCppQueue<T> {
    raw: BoostLockfreeQueue,
    phantom_data: std::marker::PhantomData<T>,
}

unsafe impl<T> Send for BoostCppQueue<T> {}
unsafe impl<T> Sync for BoostCppQueue<T> {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl<T> BoostCppQueue<T> {
    
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

impl<T> Drop for BoostCppQueue<T> {
    fn drop(&mut self) {
        unsafe { boost_queue_destroy(self.raw) };
    }
}

struct BoostCppQueueHandle<'a,T> {
    pub q: &'a BoostCppQueue<T>
}

impl<T> Handle<T> for BoostCppQueueHandle<'_,T> {
    fn push(&mut self, item: T) -> Result<(), T> {
        let ptr: *mut std::ffi::c_void = Box::<T>::into_raw(Box::new(item)) as *mut std::ffi::c_void;
        match self.q.push(ptr) {
            true => Ok(()),
            false => {
                // Really unsure if this is possible
                let reclaimed: Box<T> = unsafe { Box::from_raw(ptr as *mut T) };
                Err(*reclaimed)
            },
        }
    }

    fn pop(&mut self) -> Option<T> {
        let res = self.q.pop()?;
        let val = unsafe { Box::from_raw(res as *const T as *mut T) };
        Some(*val)
    }
}

impl<T> ConcurrentQueue<T> for BoostCppQueue<T> {
    fn register(&self) -> impl Handle<T> {
        BoostCppQueueHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("boost")
    }

    fn new(capacity: usize) -> Self {
        let raw = unsafe { boost_queue_create(capacity as u32) };
        BoostCppQueue { raw, phantom_data: std::marker::PhantomData}
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_boost_queue() {
        let q: BoostCppQueue<i32> = BoostCppQueue::new(1000);
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
        let q: BoostCppQueue<i32> = BoostCppQueue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        handle.push(2).unwrap();
        handle.push(3).unwrap();
        handle.push(4).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
        assert_eq!(handle.pop().unwrap(), 2);
        assert_eq!(handle.pop().unwrap(), 3);
        assert_eq!(handle.pop().unwrap(), 4);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: BoostCppQueue<i32> = BoostCppQueue::new(10);
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
