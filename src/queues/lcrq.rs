#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{cell::Cell, sync::atomic::{AtomicI32, Ordering}};

use crate::traits::{ConcurrentQueue, Handle};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

thread_local! {
    static THREAD_ID: Cell<Option<i32>> = const { Cell::new(None) };
}

static MAX_THREADS: i32 = 512;


// A safe Rust wrapper around the C bindings
pub struct LCRQueue {
    raw: LCRQ,
    next_thread_id: AtomicI32,
}

unsafe impl Send for LCRQueue {}
unsafe impl Sync for LCRQueue {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl LCRQueue {
    
    pub fn push(&self, item: *mut std::ffi::c_void) -> bool {
        let tid = self.get_thread_id();
        unsafe { lcrq_push(self.raw, item, tid) == 1 }
    }
    
    pub fn pop(&self) -> Option<*mut std::ffi::c_void> {
        let mut item: *mut std::ffi::c_void = std::ptr::null_mut();
        let tid = self.get_thread_id();
        let success = unsafe { lcrq_pop(self.raw, &mut item, tid) == 1 };
        if success {
            Some(item)
        } else {
            None
        }
    }
    fn get_thread_id(&self) -> i32 {
        THREAD_ID.with(|id| {
            if let Some(tid) = id.get() {
                tid
            } else {
                let new_id = self.next_thread_id.fetch_add(1, Ordering::Relaxed);
                id.set(Some(new_id));
                new_id
            }
        })
    }
    
}

impl Drop for LCRQueue {
    fn drop(&mut self) {
        unsafe { lcrq_destroy(self.raw) };

    }
}

struct LCRQHandle<'a> {
    pub q: &'a LCRQueue
}

impl Handle<Box<i32>> for LCRQHandle<'_> {
    fn push(&mut self, item: Box<i32>) -> Result<(), Box<i32>> {
        let ptr: *mut std::ffi::c_void = Box::<i32>::into_raw(item) as *mut std::ffi::c_void;
        match self.q.push(ptr) {
            true => Ok(()),
            false => {
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

impl ConcurrentQueue<Box<i32>> for LCRQueue {
    fn register(&self) -> impl crate::traits::Handle<Box<i32>> {
        LCRQHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("lcrq")
    }

    fn new(_capacity: usize) -> Self {
        THREAD_ID.with(|id| {
            id.set(Some(0));
        });
        let raw = unsafe { lcrq_create(MAX_THREADS) };
        LCRQueue { raw, next_thread_id: AtomicI32::new(1) }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_lcrq() {
        println!("creating lcrq for creation test");
        let q: LCRQueue = LCRQueue::new(1000);
        let _ = q.push(Box::<i32>::into_raw(Box::new(32)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(33)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(34)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(35)) as *mut std::ffi::c_void);
        let _ = q.push(Box::<i32>::into_raw(Box::new(36)) as *mut std::ffi::c_void);
        println!("pushed values");
        let val = unsafe {*(q.pop().unwrap() as *const i32)};
        println!("dereference value");
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
    fn register_lcrq() {
        println!("creating lcrq for register test");
        let q: LCRQueue = LCRQueue::new(1000);
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
        let q: LCRQueue = LCRQueue::new(10);
        if crate::order::benchmark_order_box(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
