use memory_management::rbfmem::Allocator;
use std::{
    ptr::null_mut,
    sync::atomic::{self, AtomicPtr},
    thread,
};

/// Basic implementation of the Michael Scott Queue.
/// The queue is a lock-free FIFO queue.
pub struct MSQueue<T> {
    /// Pointer to head of the queue
    /// which first node in the queue and the dummy node.
    head: AtomicPtr<Node<T>>,
    /// Pointer to tail of the queue.
    /// which last node in the queue and possibly the dummy node.
    tail: AtomicPtr<Node<T>>,
}

impl<T> MSQueue<T> {
    /// Creates an empty Michael Scott Queue.
    pub fn new() -> Self {
        let dummy_node = Box::into_raw(Box::new(Node::dummy_node()));
        Self {
            head: AtomicPtr::new(dummy_node),
            tail: AtomicPtr::new(dummy_node),
        }
    }

    /// Register a new handle of the queue.
    /// This method needs to be called by each thread directly.
    pub fn register(&self) -> MSQueueHandle<'_, T> {
        MSQueueHandle {
            queue: self,
            allocator: Allocator::new(),
        }
    }

    /// Enqueue `value` to the back of the queue.
    fn enqueue(&self, allocator: &mut Allocator<Node<T>, ALLOCATOR_FREE_LEN>, value: T) {
        // create the new node to be enqueued
        let new_node = allocator.allocate(Node::new(value));
        loop {
            // get current tail of queue and the next node of the tail
            let tail = unsafe { self.tail.load(atomic::Ordering::Acquire).as_mut().unwrap() };
            let next = tail.next.load(atomic::Ordering::Acquire);
            if next.is_null() {
                // tail points to the last node, try enqueueing the new node
                if tail
                    .next
                    .compare_exchange(
                        next,
                        new_node,
                        atomic::Ordering::Acquire,
                        atomic::Ordering::Acquire,
                    )
                    .is_ok()
                {
                    // enqueue was successful, update tail and finish
                    let _ = self.tail.compare_exchange(
                        tail,
                        new_node,
                        atomic::Ordering::Acquire,
                        atomic::Ordering::Acquire,
                    );
                    return;
                }
            } else {
                // tail of queue is behind, update it to point to next node
                let tail = tail as *mut Node<T>;
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    atomic::Ordering::Acquire,
                    atomic::Ordering::Acquire,
                );
            }
        }
    }

    /// Dequeue a value from the front of the queue.
    /// Returning this value or `None`, if queue is empty.
    fn dequeue(&self, allocator: &mut Allocator<Node<T>, ALLOCATOR_FREE_LEN>) -> Option<T> {
        loop {
            // get current head and tail of queue and the next node of the head
            let head = self.head.load(atomic::Ordering::Acquire);
            let tail = self.tail.load(atomic::Ordering::Acquire);
            assert!(!head.is_null());
            let next = unsafe { (*head).next.load(atomic::Ordering::Acquire) };
            if head == tail {
                // queue is empty or tail is behind, amend the situation
                if next.is_null() {
                    // queue is empty, cannot dequeue a node
                    return None;
                }
                // tail of queue is behind, update it to point to next node
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    atomic::Ordering::Acquire,
                    atomic::Ordering::Acquire,
                );
            } else {
                // head can be dequeued, try dequeueing head
                if self
                    .head
                    .compare_exchange(
                        head,
                        next,
                        atomic::Ordering::Acquire,
                        atomic::Ordering::Acquire,
                    )
                    .is_ok()
                {
                    // dequeue was successful and gained exclusive access to the value
                    let value = unsafe { (*next).value.take().unwrap() };
                    // free the memory of the previous head when it is safe to do so
                    unsafe {
                        allocator.free(head);
                    }
                    // return the taken value of the dequeued node
                    return Some(value);
                }
            }
        }
    }
}

impl<T> Drop for MSQueue<T> {
    fn drop(&mut self) {
        // drop all nodes of the queue starting from the head node
        // each node's value will be dropped if it has not been taken already
        let head = self.head.load(atomic::Ordering::Acquire);
        assert!(!head.is_null());
        let head = unsafe { Box::from_raw(head) };
        let mut next = head.next.load(atomic::Ordering::Acquire);
        // explicitly drop the nodes even, if unneccessary
        // since it may need to be freed by some allocators
        drop(head);
        while !next.is_null() {
            let node = unsafe { Box::from_raw(next) };
            next = node.next.load(atomic::Ordering::Acquire);
            drop(node);
        }
    }
}

/// Node of the Michael Scott Queue.
struct Node<T> {
    /// Next node in queue.
    next: AtomicPtr<Self>,
    /// Possible value of the node which is `None` for dummy nodes.
    value: Option<T>,
}

impl<T> Node<T> {
    /// Create a new node holding `value`.
    fn new(value: T) -> Self {
        Self {
            next: AtomicPtr::new(null_mut()),
            value: Some(value),
        }
    }

    /// Create a dummy node.
    fn dummy_node() -> Self {
        Self {
            next: AtomicPtr::new(null_mut()),
            value: None,
        }
    }
}

/// The allocator free length used by the Michael Scott Queue.
const ALLOCATOR_FREE_LEN: usize = 10;

/// A thread owned handler to the Michael Scott Queue.
pub struct MSQueueHandle<'a, T> {
    /// Reference to the queue.
    queue: &'a MSQueue<T>,
    // Thread specific epoch-based allocator.
    allocator: Allocator<Node<T>, ALLOCATOR_FREE_LEN>,
}

impl<'a, T> MSQueueHandle<'a, T> {
    /// Enqueue `value` to the back of the queue.
    pub fn enqueue(&mut self, value: T) {
        self.queue.enqueue(&mut self.allocator, value);
    }

    /// Dequeue a value from the front of the queue.
    /// Returning this value or `None`, if queue is empty.
    pub fn dequeue(&mut self) -> Option<T> {
        self.queue.dequeue(&mut self.allocator)
    }
}

/// Test uses a set of threads that enqueue and dequeue the same range of elements.
/// That means that there will be a duplicate of each element for each thread.
#[test]
fn test_rbfmem_ms_queue() {
    let queue: MSQueue<usize> = MSQueue::new();
    let thread_count = 16;
    let elements = 100_000;
    // global count for each of elements
    let mut element_counts: Vec<usize> = vec![0; elements];
    // spawn test threads
    thread::scope(|s| {
        let queue = &queue;
        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                s.spawn(|| {
                    // get a handle to concurrent queue
                    let mut queue_handle = queue.register();
                    // count for all possible elements that can be dequeued
                    let mut element_counts: Vec<usize> = vec![0; elements];
                    // state of elements to be enqueued/dequeued
                    let mut enq_num = elements;
                    let mut deq_num = elements;
                    // enqueue all the elements and dequeue the same amount of elements
                    while enq_num > 0 || deq_num > 0 {
                        if enq_num > 0 {
                            // more elements to enqueue, enqueue one more
                            queue_handle.enqueue(enq_num);
                            enq_num -= 1;
                        }
                        if deq_num > 0 {
                            // more elements to be dequeued
                            if let Some(element) = queue_handle.dequeue() {
                                // successfully dequeued an element
                                deq_num -= 1;
                                // count the occurrence of each element
                                let index = element - 1;
                                element_counts[index] += 1;
                            }
                        }
                    }
                    // return thread local count of elements
                    element_counts
                })
            })
            .collect();
        // join all the threads
        for thread in handles {
            // update the global count of each element
            let local_list = thread.join().unwrap();
            for (global, local) in element_counts.iter_mut().zip(local_list) {
                *global += local;
            }
        }
    });
    // ensure that all enqueued elements are dequeued correctly
    assert_eq!(
        element_counts.iter().sum::<usize>(),
        thread_count * elements
    );
    assert!(element_counts.iter().all(|&count| count == thread_count));
}
