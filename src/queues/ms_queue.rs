use std::mem::MaybeUninit;

use haphazard::{raw::Pointer, AtomicPtr, HazardPointer};

struct Node<T> {
    next: AtomicPtr<Node<T>>,
    data: MaybeUninit<T>,
}

pub struct MSQueue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(data: T) -> Self {
        Self { 
            next: unsafe { AtomicPtr::new(core::ptr::null_mut()) }, 
            data: MaybeUninit::new(data), 
        }
    }
    
    fn empty() -> Self {
        Self{
            next: unsafe { AtomicPtr::new(core::ptr::null_mut()) },
            data: MaybeUninit::uninit(),
        }
    }
}

impl<T: Sync + Send> MSQueue<T> {
    pub fn new() -> Self {
        let dummy = Box::new(Node::empty()).into_raw();
        Self {
            head: unsafe { AtomicPtr::new(dummy) },
            tail: unsafe { AtomicPtr::new(dummy) },
        }
    }

    pub fn enqueue(&self, hp: &mut HazardPointer, data: T) {
        let new_node: *mut Node<T> = Box::new(Node::new(data)).into_raw();
        loop {
            let current_tail: &Node<T> = self.tail.safe_load(hp).unwrap();
            let current_tail_next: *mut Node<T> = current_tail.next.load_ptr();

            if !current_tail_next.is_null() {
                // If tail already has a next
                unsafe {
                    let _ = self.tail.compare_exchange_ptr(
                        current_tail as *const Node<T> as *mut Node<T>,
                        current_tail_next
                    );
                };
            } else {
                if unsafe {
                    current_tail.next.compare_exchange_ptr(std::ptr::null_mut(), new_node)
                }
                .is_ok() {
                    unsafe {
                        let _ = self.tail.compare_exchange_ptr(current_tail as *const Node<T> as *mut Node<T>, new_node);
                    };
                    return;
                }
            }
        }
    }
    
}

