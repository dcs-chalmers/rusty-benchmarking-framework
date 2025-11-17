#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use benchmark_core::traits::{ConcurrentQueue, Handle};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// A safe Rust wrapper around the C bindings
pub struct MoodyCamelCppQueue<T> {
    raw: MoodyCamelConcurrentQueue,
    phantom_data: std::marker::PhantomData<T>
}

unsafe impl<T> Send for MoodyCamelCppQueue<T> {}
unsafe impl<T> Sync for MoodyCamelCppQueue<T> {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl<T> MoodyCamelCppQueue<T> {

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
    fn new() -> Self {
        let raw = unsafe { moody_camel_create() };
        MoodyCamelCppQueue { raw, phantom_data: std::marker::PhantomData}
    }

}

impl<T> Drop for MoodyCamelCppQueue<T> {
    fn drop(&mut self) {
        unsafe { moody_camel_destroy(self.raw) };
    }
}

struct MoodyCamelCppQueueHandle<'a,T> {
    pub q: &'a MoodyCamelCppQueue<T>
}

impl<T> Handle<T> for MoodyCamelCppQueueHandle<'_,T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        let ptr: *mut std::ffi::c_void = Box::<T>::into_raw(Box::new(item)) as *mut std::ffi::c_void;
        match self.q.push(ptr) {
            true => {
                Ok(())
            },
            false => {
                // Unsure about this
                let reclaimed_mem: Box<T> = unsafe { Box::from_raw(ptr as *mut T) };
                Err(*reclaimed_mem)
            },
        }
    }

    fn pop(&mut self) -> Option<T> {
        let res = self.q.pop()?;
        let val = unsafe { Box::from_raw(res as *const T as *mut T) };
        Some(*val)
    }
}

impl<T> ConcurrentQueue<T> for MoodyCamelCppQueue<T> {
    fn register(&self) -> impl Handle<T> {
        MoodyCamelCppQueueHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("moodycamel_cpp")
    }

    fn new(_capacity: usize) -> Self {
        MoodyCamelCppQueue::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_moody_camel() {
        let q: MoodyCamelCppQueue<i32> = MoodyCamelCppQueue::new();
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
        let q: MoodyCamelCppQueue<Box<i32>> = MoodyCamelCppQueue::new();
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
    #[ignore]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: MoodyCamelCppQueue<i32> = MoodyCamelCppQueue::new();
        if benchmark_core::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
