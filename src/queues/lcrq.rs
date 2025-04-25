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
pub struct LCRQueue<T> {
    raw: LCRQ,
    next_thread_id: AtomicI32,
    phantom_data: std::marker::PhantomData<T>,
}

unsafe impl<T> Send for LCRQueue<T> {}
unsafe impl<T> Sync for LCRQueue<T> {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl<T> LCRQueue<T> {
    
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
    fn new() -> Self {
        THREAD_ID.with(|id| {
            id.set(Some(0));
        });
        let raw = unsafe { lcrq_create(MAX_THREADS) };
        Self { raw, next_thread_id: AtomicI32::new(1), phantom_data: std::marker::PhantomData}
    } 
}

impl<T> Drop for LCRQueue<T> {
    fn drop(&mut self) {
        unsafe { lcrq_destroy(self.raw) };

    }
}

struct LCRQHandle<'a, T> {
    pub q: &'a LCRQueue<T>
}

impl<T> Handle<T> for LCRQHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T> {
        let ptr: *mut std::ffi::c_void = Box::<T>::into_raw(Box::new(item)) as *mut std::ffi::c_void;
        match self.q.push(ptr) {
            true => Ok(()),
            false => {
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

impl<T> ConcurrentQueue<T> for LCRQueue<T> {
    fn register(&self) -> impl Handle<T> {
        LCRQHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("lcrq_cpp")
    }

    fn new(_capacity: usize) -> Self {
        LCRQueue::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_lcrq() {
        println!("creating lcrq for creation test");
        let q: LCRQueue<i32> = LCRQueue::new();
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
        let q: LCRQueue<i32> = LCRQueue::new();
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
        let q: LCRQueue<i32> = LCRQueue::new();
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
