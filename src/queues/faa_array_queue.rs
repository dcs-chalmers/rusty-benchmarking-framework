use std::sync::atomic::{AtomicPtr as RawAtomicPtr, AtomicUsize, Ordering};

use haphazard::{AtomicPtr as HpAtomicPtr, HazardPointer};
use log::trace;

use crate::traits::{ConcurrentQueue, Handle};

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
            array: [const { RawAtomicPtr::new(core::ptr::null_mut()) }; SEGMENT_SIZE],
        };
        node.array[0] = RawAtomicPtr::new(data_ptr);
        node
    }

    fn empty() -> Self {
        Self {
            enqueue_index: 0.into(),
            dequeue_index: 0.into(),
            next: unsafe { HpAtomicPtr::new(core::ptr::null_mut()) },
            array: [const { RawAtomicPtr::new(core::ptr::null_mut()) }; SEGMENT_SIZE],
        }
    }
}

impl<T: Sync + Send> FAAArrayQueue<T> {
    pub fn enqueue(&self, hp: &mut HazardPointer, data: T) {
        trace!("Starting enqueue operation of FAAArrayQueue.");
        // Enqueue into boxes to make it completely generic, even if sub-optimal for raw 64-bit T
        let data_ptr = Box::<T>::into_raw(Box::new(data));
        loop {
            // load the tail of the current node via the hazard pointer (atomically)
            let tail: &Node<T> = self.tail.safe_load(hp).unwrap();
            // Increment the enqueue index in the tail
            let enq_ind = tail.enqueue_index.fetch_add(1, Ordering::SeqCst); // TODO: Is AckRel enough?

            // Is the index within the node?
            if enq_ind < SEGMENT_SIZE {
                // Within bounds, so try enqueue
                if tail.array[enq_ind]
                    .compare_exchange(
                        core::ptr::null_mut(),
                        data_ptr,
                        Ordering::Release,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    // Return as the item was enqueued
                    return;
                }
            } else {
                // Index is outside bounds of current node, so we need a new one
                let next = tail.next.load_ptr();
                if !next.is_null() {
                    // Try to update the tail pointer to this new node. Only if it still is the old tail.
                    if self
                        .tail
                        .load_ptr()
                        .eq(&(tail as *const Node<T> as *mut Node<T>))
                    {
                        unsafe {
                            let _ = self
                                .tail
                                .compare_exchange_ptr(tail as *const Node<T> as *mut Node<T>, next);
                        };
                    }
                } else {
                    // No new node avaialble, so we must create and enqueue one
                    let new_node_ptr = Box::<Node<T>>::into_raw(Box::new(Node::new(data_ptr)));

                    // First update the tail.next pointer
                    if tail.next.load_ptr().is_null()
                        && unsafe {
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

                        // The item was succesfully enqueued as part of this new node, so return
                        return;
                    } else {
                        // Could not enqueue new node, as another thread was first.
                        // We could save this for the next time. But it is a trade-off between memory efficiency and latency.
                        unsafe {
                            drop(Box::from_raw(new_node_ptr));
                        };

                        // Try to help the other thread enqueue its node. Then retry the enqueue loop
                        if self
                            .tail
                            .load_ptr()
                            .eq(&(tail as *const Node<T> as *mut Node<T>))
                        {
                            let next = tail.next.load_ptr();
                            assert!(!next.is_null());
                            unsafe {
                                let _ = self.tail.compare_exchange_ptr(
                                    tail as *const Node<T> as *mut Node<T>,
                                    next,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn dequeue(&self, hp: &mut HazardPointer) -> Option<T> {
        loop {
            // if we get the hazard pointer, we acquire and set the values below
            trace!("Entering dequeue loop");
            let head = self
                .head
                .safe_load(hp)
                .expect("Queue should never be empty");

            let current_deqs = head.dequeue_index.load(Ordering::Acquire);
            let current_enqs = head.enqueue_index.load(Ordering::Acquire);
            if current_deqs >= current_enqs {
                return None;
            }

            let deq_ind = head.dequeue_index.fetch_add(1, Ordering::SeqCst); // TODO: is AcqRel eough here?
            if deq_ind < SEGMENT_SIZE {
                // Try to dequeue a value at the index by swapping in a faulty non-null pointer (TODO: Choose a better faulty pointer)
                // TODO: For efficiency we might want to read the value first, and maybe wait a bit if it is null. Could speed up empty queues.
                let dequeued =
                    &head.array[deq_ind].swap(1u64 as *mut u64 as *mut T, Ordering::AcqRel);
                if !dequeued.is_null() {
                    // Dequeued a real value, so return it
                    return unsafe { Some(dequeued.read()) };
                }
            } else {
                // Dequeue index outside of the bounds, so try to move on to the next node, or return None
                let next = head.next.load_ptr();
                if next.is_null() {
                    // Return empty as the current head is empty and there is no next node
                    return None;
                } else {
                    // Update the head pointer to its next node. Only do it if not already done
                    // Could also try to help the enqueuers here, but is not needed.
                    if self
                        .head
                        .load_ptr()
                        .eq(&(head as *const Node<T> as *mut Node<T>))
                    {
                        unsafe {
                            let _ = self
                                .head
                                .compare_exchange_ptr(head as *const Node<T> as *mut Node<T>, next);
                        }
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
            // Drop the data inside the arrays between the dequeue and enqueue indexes
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
        String::from("faa_array_queue")
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
