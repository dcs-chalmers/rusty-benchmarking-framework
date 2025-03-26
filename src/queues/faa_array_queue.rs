use std::sync::atomic::{AtomicPtr as RawAtomicPtr, AtomicUsize, Ordering};

use haphazard::{AtomicPtr as HpAtomicPtr, HazardPointer};
use log::{error, trace};

use crate::{ConcurrentQueue, Handle};

const SEGMENT_SIZE: usize = 1024;

struct Node<T> {
    enqueue_index: AtomicUsize, // Can change these counts to smaller ints
    dequeue_index: AtomicUsize, // Can also add cache padding
    next: HpAtomicPtr<Node<T>>,
    array: [RawAtomicPtr<T>; SEGMENT_SIZE], // TODO: Want to make a queue version for 64 bit values, to avoid box indirections
}

#[derive(Debug)]
pub struct FAAArrayQueue<T> {
    head: HpAtomicPtr<Node<T>>,
    tail: HpAtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(data_ptr: *mut T) -> Self {
        let mut node = Self {
            enqueue_index: 1.into(),
            dequeue_index: 0.into(),
            next: unsafe { HpAtomicPtr::new(core::ptr::null_mut()) },
            array: [const { RawAtomicPtr::new(core::ptr::null_mut()) }; SEGMENT_SIZE], // What does the const add here? Why does it fix the copy issue?
        };
        node.array[0] = RawAtomicPtr::new(data_ptr);
        node
    }

    fn empty() -> Self {
        Self {
            enqueue_index: 0.into(),
            dequeue_index: 0.into(),
            next: unsafe { HpAtomicPtr::new(core::ptr::null_mut()) },
            array: [const { RawAtomicPtr::new(core::ptr::null_mut()) }; SEGMENT_SIZE], // What does the const add here? Why does it fix the copy issue?
        }
    }
}

impl<T: Sync + Send> FAAArrayQueue<T> {
    pub fn enqueue(&self, hp: &mut HazardPointer, data: T) {
        trace!("Starting enqueue operation of FAAArrayQueue.");
        // Don't want to enqueue a box, but sort of have to to get a generic queue
        let data_ptr = Box::<T>::into_raw(Box::new(data));
        loop {
            // load the tail of the current node via the hazard pointer (atomically)
            let tail: &Node<T> = self.tail.safe_load(hp).unwrap();
            // Increment the enqueue index in the tail
            let enq_ind = tail.enqueue_index.fetch_add(1, Ordering::SeqCst); // TODO: memory ordering

            // Try to insert item if within bounds, otherwise try to enqueue new node
            if enq_ind < SEGMENT_SIZE {
                // Within bounds, so try enqueue
                let enq_cell: &RawAtomicPtr<T> = &tail.array[enq_ind];
                if enq_cell
                    .compare_exchange(
                        core::ptr::null_mut(),
                        data_ptr,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    // TODO: Ordering
                    return;
                }
            } else {
                // Outside bounds, so need new node
                let next = tail.next.load_ptr();
                if !next.is_null() {
                    // try to set it as the queue tail (should check before)
                    unsafe {
                        let _ = self
                            .tail
                            .compare_exchange_ptr(tail as *const Node<T> as *mut Node<T>, next);
                    };
                } else {
                    // Allocate a new node and swap it in
                    let new_node_ptr = Box::<Node<T>>::into_raw(Box::new(Node::new(data_ptr)));
                    // First update the next pointer
                    if unsafe {
                        tail.next
                            .compare_exchange_ptr(core::ptr::null_mut(), new_node_ptr)
                    }
                    .is_ok()
                    {
                        // Try to complete the enqueue by uptating the tail pointer
                        unsafe {
                            let _ = self.tail.compare_exchange_ptr(
                                tail as *const Node<T> as *mut Node<T>,
                                new_node_ptr,
                            );
                        }
                        // The item was succesfully enqueued
                        return;
                    } else {
                        // Did not go through, so de-alloc new_node, and try loop again.
                        // We could save this for the next time. But it is a trade-off between memory efficiency and latency.
                        unsafe {
                            drop(Box::from_raw(new_node_ptr));
                        };
                    }
                }
            }
        }
    }

    pub fn dequeue(&self, hp: &mut HazardPointer) -> Option<T> {
        loop {
            // if we get the hazard pointer, we acquire and set the values below
            trace!("Entering dequeue loop");
            let head = match self.head.safe_load(hp) {
                Some(v) => v,
                None => {
                    error!("Queue should never be empty");
                    panic!("Queue should never be empty");
                }
            };
            let current_deqs = head.dequeue_index.load(Ordering::Acquire);
            let current_enqs = head.enqueue_index.load(Ordering::Acquire);
            if current_deqs >= current_enqs {
                return None;
            }

            let deq_ind = head.dequeue_index.fetch_add(1, Ordering::SeqCst);
            if deq_ind < SEGMENT_SIZE {
                let deq_cell: &RawAtomicPtr<T> = &head.array[deq_ind];
                let dequeued = deq_cell.swap(1u64 as *mut u64 as *mut T, Ordering::SeqCst);
                if !dequeued.is_null() {
                    return unsafe { Some(dequeued.read()) };
                }
            } else {
                let next = head.next.load_ptr();
                if next.is_null() {
                    // Return empty as current is empty and there is no next
                    return None;
                } else {
                    // Update the head pointer to the new node.
                    // Could also try to help the enqueuers here, but is not needed.
                    unsafe {
                        let _ = self.head.compare_exchange_ptr(
                            head as *const Node<T> as *mut Node<T>,
                            head.next.load_ptr() as *const Node<T> as *mut Node<T>,
                        );
                    };
                }
            }
        }
    }
}

impl<T> Drop for FAAArrayQueue<T> {
    fn drop(&mut self) {
        trace!("Starting drop FAAArrayQueue");
        // Transform to box to transfer ownership back to Rust's memory management system
        let head: Box<Node<T>> = unsafe { Box::from_raw(self.head.load_ptr()) };
        let mut next = head.next;

        while !next.load_ptr().is_null() {
            let node: Box<Node<T>> = unsafe { Box::from_raw(next.load_ptr()) };
            // Drop the data
            for data in node
                .array
                .into_iter()
                .take(node.enqueue_index.load(Ordering::SeqCst))
                .skip(node.dequeue_index.load(Ordering::SeqCst))
            {
                data.load(Ordering::SeqCst);
            }

            next = node.next;
        }
        trace!("Done dropping");
    }
}

pub struct FAAArrayQueueHandle<'a, T> {
    queue: &'a FAAArrayQueue<T>,
    hp: HazardPointer<'static>,
}

impl<T: Sync + Send> ConcurrentQueue<T> for FAAArrayQueue<T> {
    fn register(&self) -> impl Handle<T> {
        FAAArrayQueueHandle {
            queue: self,
            hp: HazardPointer::new(),
        }
    }
    fn get_id(&self) -> String {
        String::from("FAAArrayQueue")
    }
    fn new(_size: usize) -> Self {
        let sentinel = Box::into_raw(Box::new(Node::empty()));
        Self {
            head: unsafe { HpAtomicPtr::new(sentinel) },
            tail: unsafe { HpAtomicPtr::new(sentinel) },
        }
    }
}

impl<T: Sync + Send> Handle<T> for FAAArrayQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T> {
        self.queue.enqueue(&mut self.hp, item);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.dequeue(&mut self.hp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_faaaq_queue() {
        let q: FAAArrayQueue<i32> = FAAArrayQueue::new(1000);
        let mut hp = HazardPointer::new();
        q.enqueue(&mut hp, 1);
    }

    #[test]
    fn register_faaaq_queue() {
        let q: FAAArrayQueue<i32> = FAAArrayQueue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: FAAArrayQueue<i32> = FAAArrayQueue::new(10);
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
