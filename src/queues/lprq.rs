#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{cell::Cell, sync::atomic::{AtomicI32, Ordering}};

use crate::{ConcurrentQueue, Handle};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

thread_local! {
    static THREAD_ID: Cell<Option<i32>> = const { Cell::new(None) };
}

static MAX_THREADS: i32 = 512;


// A safe Rust wrapper around the C bindings
pub struct LPRQueue {
    raw: LPRQ,
    next_thread_id: AtomicI32,
}

unsafe impl Send for LPRQueue {}
unsafe impl Sync for LPRQueue {}

impl LPRQueue {
    
    pub fn push(&self, item: *mut std::ffi::c_void) -> bool {
        let tid = self.get_thread_id();
        unsafe { lprq_push(self.raw, item, tid) == 1 }
    }
    
    pub fn pop(&self) -> Option<*mut std::ffi::c_void> {
        let mut item: *mut std::ffi::c_void = std::ptr::null_mut();
        let tid = self.get_thread_id();
        let success = unsafe { lprq_pop(self.raw, &mut item, tid) == 1 };
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

impl Drop for LPRQueue {
    fn drop(&mut self) {
        unsafe { lprq_destroy(self.raw) };

    }
}

struct LPRQHandle<'a> {
    pub q: &'a LPRQueue
}

impl Handle<Box<i32>> for LPRQHandle<'_> {
    fn push(&mut self, item: Box<i32>) -> Result <(), Box<i32>>{
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

impl ConcurrentQueue<Box<i32>> for LPRQueue {
    fn register(&self) -> impl crate::Handle<Box<i32>> {
        LPRQHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("lprq")
    }

    fn new(_capacity: usize) -> Self {
        THREAD_ID.with(|id| {
            id.set(Some(0));
        });
        let raw = unsafe { lprq_create(MAX_THREADS) };
        LPRQueue { raw, next_thread_id: AtomicI32::new(1) }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_lprq() {
        println!("creating lprq for creation test");
        let q: LPRQueue = LPRQueue::new(1000);
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
    fn register_lprq() {
        println!("creating lprq for register test");
        let q: LPRQueue = LPRQueue::new(1000);
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
}

