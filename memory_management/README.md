# Memory Management Library
This library of Rusty Benchmarking Framework contains functionality
for simplifying memory management in concurrent data structures.

## RBFMEM
RBFMEM is a minimalistic epoch-based memory allocator and reclamator.
RBFMEM builds on the ideas from [`ssmem`](https://github.com/LPD-EPFL/ssmem)
developed at [EPFL](https://www.epfl.ch/en/).
RBFMEM extends it by using Rust's type system by allowing for threads to terminate
asynchronously by waiting until the thread's data is safe to be reclaimed.
It also offers threads to start asynchronously without the need for barriers.

Epoch-based memory reclamation ensures that memory from a globally shared data structure
is not reclaimed when references to this memory could exist in other threads.
This is important for lock-free data structures, since many threads could parse
the shared data structure at the same time and when removing an object
another thread might be traversing this very object. Reclaiming this object at this time
could lead to use-after-free bugs and other undefined behaviour.
Epoch-based memory reclamation relies on the fact that once a shared object has been removed
from a globally shared data structure no new local references can be created to this object
by other threads. After this point, the scheme only needs to ensure that the previously
created references by other threads have been defunct. This is done by timestamping each thread
using logical clocks when operations have been completed.

### Usage
In order to use RBFMEM, the following import could be used:
```rs
use memory_management::rbfmem::{self, Allocator};
```

#### Creating a new allocator
Create a new thread local allocator that allocates instances of the type `T` and
keeps `LEN` number of new elements before trying to reclaim memory.
```rs
let mut allocator: Allocator<T, LEN> = Allocator::new();
```
All threads with an allocator will synchronize with each other
to safely reclaim memory when it is possible using timestamps.
These timestamps will be incremented when allocating new memory or freeing memory.


> **IMPORTANT!**
>
> All threads that access objects from a shared data structure that uses RBFMEM must have
a thread local allocator.
Otherwise, RBFMEM cannot know of the thread's existance and thus memory can be reclaimed
while this thread is accessing the memory.

#### Allocating memory
Allocate a new instance of type `T` and assigning it the value of `val`.
This returns a mutable raw pointer to type `T` (`*mut T`).
```rs
let obj = allocator.allocate(val);
```
If an initializing value is undesirable, then `Option` or `mem::MaybeUninit`
could be used as the type `T` for the allocator and providing the `allocate` method
with the value of `None` or `mem::MaybeUninit::uninit()`.
However `Option` is to be preferred, as `mem::MaybeUninit` can lead to undefined behaviour.

#### Freeing memory
Free the memory of the pointer `obj`, when it is safe to do so.
This function is marked as `unsafe`, since the programmer is solely responsible
for ensuring its safety conditions:
- `free` must only be called once per object, as it otherwise will result in a double free.
This means that the programmer must ensure that only one thread can be seen as
the one that removed the object from the shared data structure
and is the thread responsible for calling `free`.

- The memory of `obj` needs to come from either an allocator of type `Allocator<T, _>`
or from `rbfmem::preallocate::<T>()` where `T` is the type of the allocated object.
```rs
allocator.free(obj);
```

#### Read only threads
Some threads may not remove or add new objects to a shared data structure, but only parse and read
the data structure. However, this would lead to other threads not noticing when the thread is done
parsing shared objects, since the timestamp will not be incremented.
To solve this, the programmer can call `safe_to_reclaim_memory()` to indicate when the thread
with an allocator is done with old object references.
This function should only be used by read only threads, and not to improve performance.
```rs
rbfmem::safe_to_reclaim_memory();
```

#### Preallocating memory
Sometimes a shared data structure needs to be prepopulated with data.
This can be done with the function `preallocate()` without the need of a timestamping allocator.
This is useful when initializing shared data structures from the main thread, since the other threads
do not need to keep the main thread into account when comparing timestamps of each other.
Preallocate a new instance initialized with the value of `val` of type `T`.
This returns a mutable raw pointer to type `T` (`*mut T`).
```rs
let obj = rbfmem::preallocate(val);
```

#### Dropping shared data structures
It can be useful for the main thread to be able to remove objects when
dropping a shared data structure, after all threads have terminated.
This can be done using the function `take_ownership()`, which takes ownership of the object value
from the pointer `obj` returning the value wrapped in an owned type.
This function is marked as `unsafe`, since the programmer is solely responsible
for ensuring its safety conditions:
- `take_ownership` must only be called once per object, as it otherwise will result in a double free.

- The memory of `obj` needs to come from either an allocator of type `Allocator<T, _>`
or from `rbfmem::preallocate::<T>()`.
```rs
let val = rbfmem::take_ownership(obj);
```

#### Terminating or resetting RBFMEM
Sometimes it might be necessary to reset the global state of RBFMEM for all threads.
For example, when terminating the entire program or between iterations of benchmarks.
This can be done using the function `global_reset()`.
However, RBFMEM cannot know when this function can be called safely.
The function thus assumes that there are no allocators currently existing in any threads.
Thus, this function is marked as `unsafe`, since the programmer is solely responsible
for ensuring its safety conditions:
- All running threads which could use an allocator have been terminated.
- The destructors of all allocators have been called.

Failing to meet these conditions will result in undefined behaviour and use-after free bugs.
```rs
rbfmem::global_reset();
```

### Example
Here follows a simple example of how RBFMEM is supposed to
be used by a concurrent data structure and in this case
the [Michael Scott Queue](https://dl.acm.org/doi/10.1145/248052.248106):

```rs
use memory_management::rbfmem::{self, Allocator};
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
        let dummy_node = rbfmem::preallocate(Node::dummy_node());
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
        let mut next = self.head.load(atomic::Ordering::Acquire);
        while !next.is_null() {
            let node = unsafe { rbfmem::take_ownership(next) };
            next = node.next.load(atomic::Ordering::Acquire);
            drop(node);
        }
        // IMPORTANT!
        // resetting RBFMEM here works well for the benchmarking framework
        // but should be done elsewhere in actual production code
        unsafe {
            rbfmem::global_reset();
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
fn main() {
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
```
