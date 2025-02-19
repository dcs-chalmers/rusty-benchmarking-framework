use std::mem::MaybeUninit;

use haphazard::{raw::Pointer, AtomicPtr, HazardPointer};

use crate::{ConcurrentQueue, Handle};

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


    pub fn enqueue(&self, hp: &mut HazardPointer, data: T) {
        // first off we create a new node where we put the data on the heap (not the stack) and turn it into a pointer
        let new_node: *mut Node<T> = Box::new(Node::new(data)).into_raw();
        loop {
            // load the tail of the current node via the hazard pointer (atomically)
            let current_tail: &Node<T> = self.tail.safe_load(hp).unwrap();
            // load the current tails next pointer (atomically)
            let current_tail_next: *mut Node<T> = current_tail.next.load_ptr();

            if !current_tail_next.is_null() {
                // If tail already has a next
                unsafe {
                    // we swap the current tail to point to its next node
                    let _ = self.tail.compare_exchange_ptr(
                        current_tail as *const Node<T> as *mut Node<T>,
                        current_tail_next
                    );
                };
            } else {
                // if the tail dont have a next
                if unsafe {
                    // CAS if pointer is null -> set null pointer to the new node
                    current_tail.next.compare_exchange_ptr(std::ptr::null_mut(), new_node)
                }
                .is_ok() {
                    unsafe {
                        // CAS current tail to the new node 
                        let _ = self.tail.compare_exchange_ptr(current_tail as *const Node<T> as *mut Node<T>, new_node);
                    };
                    return;
                }
            }
        }
    }

    pub fn dequeue(&self, hp_head: &mut HazardPointer, hp_next: &mut HazardPointer) -> Option<T> {
        loop {
            // if we get the hazard pointer, we acquire and set the values below
            let head = self
                .head
                .safe_load(hp_head)
                .expect("MS queue should never be empty");
            let head_ptr = head as *const Node<T> as *mut Node<T>;
            let next_ptr = head.next.load_ptr();
            let tail_ptr = self.tail.load_ptr();

            // non empty queue
            if head_ptr != tail_ptr {

                // get next via hazard poinetr
                let next = head.next.safe_load(hp_next).unwrap();
                // if CAS gets an OK we update head pointer to the next pointer and retire the old head pointer
                if let Ok(unlinked_head_ptr) = unsafe {
                    self.head.compare_exchange_ptr(head_ptr, next_ptr)
                } {
                    unsafe {
                        unlinked_head_ptr.unwrap().retire();
                    }
                    // return the value of the new head
                    return Some(unsafe {std::ptr::read(next.data.assume_init_ref() as *const _)}); // 1,2,3,4,5 -> dequeue: 1 | 2,3,4,5 -> return 2?????
                }
                // the queue is empty but another thread has enqueued 
                else if !next_ptr.is_null() {  
                    // help with the enqueue via CAS tail pointer to next pointer
                    unsafe {
                        let _ = self.tail.compare_exchange_ptr(tail_ptr as *mut Node<T>, next_ptr);
                    }
                } 
                // queue is empty
                else {
                    return None;
                }
            }
        }
    }
    
}

impl<T> Drop for MSQueue<T> {
    fn drop(&mut self) {
        // Transform to box to transfer ownership back to Rust's memory management system
        let head = unsafe {
            Box::from_raw(self.head.load_ptr())
        };
        let mut next = head.next;


        while !next.load_ptr().is_null(){
            let node = unsafe {
                Box::from_raw(next.load_ptr())
            };
            // Drop the data
            unsafe {node.data.assume_init()};

            next = node.next;
        }
    }
}


pub struct MSQueueHandle<'a, T> {
    queue: &'a MSQueue<T>,
    hp1: HazardPointer<'a>,
    hp2: HazardPointer<'a>, 
}

impl<T: Sync + Send> ConcurrentQueue<T> for MSQueue<T> {
    fn register(&self) -> impl Handle<T> {
        MSQueueHandle {
            queue: &self,
            hp1: HazardPointer::new(), 
            hp2: HazardPointer::new(),
        }
    }
    fn get_id(&self) -> String {
        String::from("MSQueue")
    }
    fn new(_size: usize) -> Self {
        let dummy = Box::new(Node::empty()).into_raw();
        Self {
            head: unsafe { AtomicPtr::new(dummy) },
            tail: unsafe { AtomicPtr::new(dummy) },
        }
    }
}

impl<T: Sync + Send> Handle<T> for MSQueueHandle<'_, T> {
    fn push(&mut self, item: T) {

        self.queue.enqueue(&mut self.hp1, item);
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.dequeue(&mut self.hp1, &mut self.hp2)
        
    }
}