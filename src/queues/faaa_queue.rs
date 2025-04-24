use std::{ptr::null_mut, sync::atomic::{AtomicPtr as RawAtomicPtr, AtomicUsize, Ordering::SeqCst}};

use haphazard::{raw::Pointer, AtomicPtr as HpAtomicPtr, HazardPointer};
use log::trace;
use crossbeam::utils::CachePadded;
use crate::traits::{ConcurrentQueue, Handle};

const BUFFER_SIZE: usize = 1024;

struct Node<T> {
    enqueue_index: CachePadded<AtomicUsize>,
    dequeue_index: CachePadded<AtomicUsize>,
    next: CachePadded<HpAtomicPtr<Node<T>>>,
    array: [RawAtomicPtr<T>; BUFFER_SIZE],
}

#[derive(Debug)]
pub struct FAAAQueue<T> {
    head: HpAtomicPtr<Node<T>>,
    tail: HpAtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(data_ptr: *mut T) -> Self {
        let mut node = Self {
            enqueue_index: CachePadded::new(1.into()),
            dequeue_index: CachePadded::new(0.into()),
            next: unsafe { CachePadded::new(HpAtomicPtr::new(core::ptr::null_mut())) },
            array: [const { RawAtomicPtr::new(core::ptr::null_mut()) }; BUFFER_SIZE],
        };
        // NOTE: Copies the address.
        node.array[0] = RawAtomicPtr::new(data_ptr);
        node
    }

    fn empty() -> Self {
        Self {
            enqueue_index: CachePadded::new(0.into()),
            dequeue_index: CachePadded::new(0.into()),
            next: unsafe { CachePadded::new(HpAtomicPtr::new(core::ptr::null_mut())) },
            array: [const { RawAtomicPtr::new(core::ptr::null_mut()) }; BUFFER_SIZE],
        }
    }
}
impl<T> FAAAQueue<T> {
    fn enqueue(&self, item: T, mut hp: HazardPointer) {
        let item_ptr = Box::into_raw(Box::new(item));
        loop {
            let ltail = self.tail.safe_load(&mut hp).unwrap();
            let idx = ltail.enqueue_index.fetch_add(1, SeqCst);
            if idx > BUFFER_SIZE - 1 { // This node is full.
                if ltail as *const _ != self.tail.load_ptr() {continue;}
                let lnext: *mut Node<T> = ltail.next.load_ptr();
                if lnext.is_null() {
                    // NOTE: Must copy item_ptr? Otherwise it would be moved
                    // out of scope?
                    let new_node = Box::into_raw(Box::new(Node::new(item_ptr)));
                    if unsafe { ltail.next.compare_exchange_ptr(null_mut(), new_node).is_ok() } {
                        let _ = unsafe { self.tail.compare_exchange_ptr(ltail as *const _ as *mut _, new_node) };
                        hp.reset_protection();
                        return;
                    }
                    // NOTE: Fine since it is dropping the pointer to item,
                    // which is a copy of item_ptr?
                    unsafe { drop(Box::from_raw(new_node)) };
                } else {
                    // Help other thread enqueue?
                    let _ = unsafe { self.tail.compare_exchange_ptr(ltail as *const _ as *mut _, lnext) };
                }
                continue;
            }
            let item_null: *mut T = null_mut();
            if ltail.array[idx].compare_exchange(item_null, item_ptr, SeqCst, SeqCst).is_ok() {
                hp.reset_protection();
                return;
            }
        }
    }
}
