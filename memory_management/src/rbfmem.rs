use std::{
    cell::Cell,
    mem,
    ptr::null_mut,
    sync::atomic::{self, AtomicPtr, AtomicUsize},
};

// Global state shared by all threads for memory management
/// The next available thread id for memory management
static GLOBAL_NEXT_ID: AtomicUsize = AtomicUsize::new(0);
/// Global linked list of thread timestamp nodes
static GLOBAL_TIMESTAMPS: AtomicPtr<ThreadTimeStamp> = AtomicPtr::new(null_mut());

// Thread local state for memory management
thread_local! {
    /// Pointer to the current thread's own timestamp node
    static LOCAL_TIMESTAMP: Cell<*mut ThreadTimeStamp> = const { Cell::new(null_mut()) };
    /// The number of allocators the current thread has
    static LOCAL_ALLOCATOR_COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Clean up the global state of RBFMEM for all threads.
/// This is useful before exiting the program or between iterations of benchmarks.
/// # Safety
/// The function assumes that there are no allocators currently existing in any threads.
/// Thus, this function may only be safely called when the programmer knows that:
/// - All running threads which could use an allocator have been terminated.
/// - The destructors of all allocators have been called.
///
/// Failing to meet these conditions will result in undefined behaviour and use-after free bugs.
pub unsafe fn global_reset() {
    // reset the global thread id counter back to zero
    GLOBAL_NEXT_ID.store(0, atomic::Ordering::Release);
    // reset thread timestamps linked list to be empty
    let mut timestamps_head = GLOBAL_TIMESTAMPS.swap(null_mut(), atomic::Ordering::Acquire);
    // drop the previous thread timestamps linked list starting from the head of the list
    while !timestamps_head.is_null() {
        let node = unsafe { Box::from_raw(timestamps_head) };
        timestamps_head = node.next;
        drop(node);
    }
}

/// Preallocate with RBFMEM a new object of type `T` with assigned `value`
/// returning a mutable reference guaranteed to be non null to this memory.
/// This is useful for initializing data structures that use RBFMEM for memory management.
/// # Important!
/// The developer is solely responsible for giving the memory back to an allocator,
/// otherwise the program will leak memory.
pub fn preallocate<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}

/// Takes ownership of the `obj` returning the value wrapped in an owned type.
/// This is useful for dropping a shared data structure, from the main thread.
/// # Safety
/// - The programmer is solely responsible for only calling `take_ownership`
/// once per object as it otherwise will result in a double free.
/// - The programmer must only call `take_ownership` on memory allocated
/// by the allocator or `rbfmem::preallocate()`.
pub unsafe fn take_ownership<T>(obj: *mut T) -> Box<T> {
    unsafe {
        // better to use `Box::into_inner()`, but it is experimental
        Box::from_raw(obj)
    }
}

/// Indicate that the current thread is done with access to shared global data
/// and that memory previously freed by other threads now can be safely reclaimed.
/// This is useful for threads that do not modify shared global data but reads it
/// ensuring that memory is being reclaimed.
/// # Panics
/// This function will panic if the current thread has not joined global garbage collection.
pub fn safe_to_reclaim_memory() {
    let timestamp = LOCAL_TIMESTAMP.get();
    assert!(
        !LOCAL_TIMESTAMP.get().is_null(),
        "Each thread must have at least one allocator to join the global garbage collection",
    );
    unsafe {
        (*timestamp).version.increment();
    }
}

/// A thread local Allocator that allocates instances of type `T`.
/// The allocator synchronizes with other threads when freeing memory
/// by using a minimalistic Epoch-based memory reclamation scheme.
///
/// The allocator keeps lists of objects to free, the number of which
/// can be specified by `FREE_LEN`.
pub struct Allocator<T, const FREE_LEN: usize> {
    /// The head of the linked list of free lists.
    /// Which stores objects to be freed when it is safe.
    free_list: Box<FreeList<T, FREE_LEN>>,
    /// The length of the linked list of free lists.
    free_list_len: usize,
    /// The head of the linked list of collected lists.
    /// Which stores objects that have been freed and can be reused.
    collected_list: Option<Box<FreeList<T, FREE_LEN>>>,
    /// The length of the linked list of collected lists.
    collected_list_len: usize,
    /// The head of the linked list of available lists.
    /// Which stores lists that can be reused.
    available_list: Option<Box<FreeList<T, FREE_LEN>>>,
}

impl<T, const FREE_LEN: usize> Allocator<T, FREE_LEN> {
    // Public API:
    /// Allocate a new thread specific allocator for allocating objects of type `T`.
    /// Where `FREE_LEN` is the length of each free list.
    pub fn new() -> Self {
        // join gc if not already
        Self::thread_join_gc();
        // create new allocator
        Self {
            free_list: Box::new(FreeList::new(None)),
            free_list_len: 1,
            collected_list: None,
            collected_list_len: 0,
            available_list: None,
        }
    }

    /// Allocate a new object of type `T` with assigned `value` returning
    /// a mutable reference guaranteed to be non null to this memory.
    /// # Important!
    /// The developer is solely responsible for giving the memory back to an allocator,
    /// otherwise the program will leak memory.
    pub fn allocate(&mut self, value: T) -> *mut T {
        // allocate a new object
        let obj = if let Some(head) = self.collected_list.as_mut() {
            // reuse an old object stored in allocator
            head.object_index -= 1;
            let obj = mem::replace(&mut head.objects[head.object_index], null_mut());
            // replace and drop the old value at not null reused memory
            assert!(!obj.is_null());
            unsafe {
                let old = std::ptr::replace(obj, value);
                drop(old);
            }
            // check if the collected list has been completely consumed
            if head.object_index == 0 {
                // put the next head of collected lists as the new head
                let next = head.next.take();
                let head = self.collected_list.take().unwrap();
                self.collected_list = next;
                // reuse the old head of collected lists as a new available list
                // (to later be used for a new free list)
                self.make_available(head);
                self.collected_list_len -= 1;
            }
            obj
        } else {
            // no memory that can be reused is available, allocate new memory
            Box::into_raw(Box::new(value))
        };
        // take a time step
        safe_to_reclaim_memory();
        obj
    }

    /// Free the `obj` to the allocator when it is safe.
    /// # Safety
    /// - The programmer is solely responsible for only calling `free`
    /// once per object as it otherwise will result in a double free.
    /// - The programmer must only call free on memory allocated by
    /// the allocator or `rbfmem::preallocate()`.
    pub unsafe fn free(&mut self, obj: *mut T) {
        // check if current free list is full
        if self.free_list.object_index == self.free_list.objects.len() {
            // time stamp current free list
            self.free_list.make_timestamp();
            self.reclaim_memory();
            // add a new free list as the new head of the linked list of free lists
            let mut new_free_list = self.take_available();
            mem::swap(&mut self.free_list, &mut new_free_list);
            self.free_list.next = Some(new_free_list);
            self.free_list_len += 1;
        }
        // add the object to the free list (to be freed later)
        self.free_list.objects[self.free_list.object_index] = obj;
        self.free_list.object_index += 1;
        // take a time step
        safe_to_reclaim_memory();
    }

    // Internal methods:
    /// Try to reclaim memory when it is safe.
    fn reclaim_memory(&mut self) {
        if let Some(next) = self.free_list.next.as_ref()
            && !next.timestamps.is_empty()
            && !self.free_list.timestamps.is_empty()
            && self.free_list.is_newer_than(next)
        {
            // the following free lists after the head are safe to be reclaimed,
            // remove these free lists
            let list_count = self.free_list_len - 1;
            let next = self.free_list.next.take();
            self.free_list_len = 1;
            // move these free lists to the tail of collected lists (to be reused later when allocating)
            if let Some(collected_list) = self.collected_list.as_mut() {
                collected_list.tail().next = next;
            } else {
                self.collected_list = next;
            }
            self.collected_list_len += list_count;
        }
    }

    /// Current thread announces its existance to all other threads in order
    /// to later perform garbage collection.
    fn thread_join_gc() {
        // update thread local allocator count
        LOCAL_ALLOCATOR_COUNT.set(LOCAL_ALLOCATOR_COUNT.get() + 1);
        // check if thread has not already joined garbage collection
        if LOCAL_TIMESTAMP.get().is_null() {
            // get an id for the thread
            let id = GLOBAL_NEXT_ID.fetch_add(1, atomic::Ordering::Acquire);
            // create a new time stamp object for current thread
            let mut current = GLOBAL_TIMESTAMPS.load(atomic::Ordering::Acquire);
            let thread_timestamp = Box::into_raw(Box::new(ThreadTimeStamp {
                id,
                version: TimeValue::new(0),
                next: current,
            }));
            LOCAL_TIMESTAMP.set(thread_timestamp);
            // try append current thread as the new head of global list
            // of thread timestamps until CAS succeeds
            while GLOBAL_TIMESTAMPS
                .compare_exchange(
                    current,
                    thread_timestamp,
                    atomic::Ordering::Acquire,
                    atomic::Ordering::Acquire,
                )
                .is_err()
            {
                current = GLOBAL_TIMESTAMPS.load(atomic::Ordering::Acquire);
                unsafe {
                    (*thread_timestamp).next = current;
                }
            }
        }
    }

    /// Make used `collected_list` an available list.
    fn make_available(&mut self, collected_list: Box<FreeList<T, FREE_LEN>>) {
        // reset the collected list to be used as a available list
        let mut new_available = collected_list;
        new_available.object_index = 0;
        // push new available list to the front of the available lists
        new_available.next = self.available_list.take();
        self.available_list = Some(new_available);
    }

    /// Take an available list or allocate a new one to be used as a new free list.
    fn take_available(&mut self) -> Box<FreeList<T, FREE_LEN>> {
        // check if an available list exists, then take the head
        if let Some(mut free_list) = self.available_list.take() {
            // update available lists to be the lists following the head
            self.available_list = free_list.next.take();
            // reset the head
            free_list.object_index = 0;
            free_list
        } else {
            // no available list exists, allocate a new free list
            Box::new(FreeList::new(None))
        }
    }
}

impl<T, const FREE_LEN: usize> Drop for Allocator<T, FREE_LEN> {
    fn drop(&mut self) {
        // begin thread local allocator destruction
        let allocator_count = LOCAL_ALLOCATOR_COUNT.get() - 1;
        LOCAL_ALLOCATOR_COUNT.set(allocator_count);
        // mark thread as finished, when all its allocators have been detroyed
        let thread_timestamp = LOCAL_TIMESTAMP.get();
        if allocator_count == 0 && !thread_timestamp.is_null() {
            unsafe {
                (*thread_timestamp).version.mark_as_finished();
            }
        }
        // timestamp current free list head
        self.free_list.make_timestamp();
        // wait for data to be freed to be unused by other threads
        let mut timestamps: Vec<TimeValue> = Vec::new();
        while self.free_list.is_in_use(&mut timestamps) {
            std::hint::spin_loop();
        }
        // all owned data can now be safely dropped
    }
}

/// Thread specific timestamp node.
#[derive(Clone, Copy)]
struct ThreadTimeStamp {
    /// Thread id for memory management.
    id: usize,
    /// Current timestamp value.
    version: TimeValue,
    /// Next thread timestamp node in
    /// the global linked list of thread timestamps.
    next: *mut ThreadTimeStamp,
}

/// Timestamp value used for Epoch-based memory reclamation comparisons.
#[derive(Clone, Copy, Debug)]
enum TimeValue {
    /// Active thread timestamp with a value.
    Active(usize),
    /// Thread has finished its execution and its time value can be ignored.
    Finished,
}

impl TimeValue {
    /// Create a new time value starting from `start_time`.
    pub fn new(start_time: usize) -> Self {
        TimeValue::Active(start_time)
    }

    /// Increment current time value one step, if active.
    pub fn increment(&mut self) {
        if let TimeValue::Active(time) = self {
            *time += 1;
        }
    }

    /// Mark timestamp as finished meaning that this time value
    /// does not matter when comparing with other timestamps.
    pub fn mark_as_finished(&mut self) {
        *self = TimeValue::Finished;
    }

    /// Checks if `self` has a newer time value than `other`.
    pub fn is_newer(&self, other: &Self) -> bool {
        match (self, other) {
            (TimeValue::Active(a), TimeValue::Active(b)) => a > b,
            // if a time value is finished in any case, then discard it
            _ => true,
        }
    }
}

/// Free list of length `SIZE` with objects of type `T`.
/// Used for storing object adresses to be freed or reused.
struct FreeList<T, const SIZE: usize> {
    /// Next free list in the linked list of lists.
    next: Option<Box<FreeList<T, SIZE>>>,
    /// Timestamp of free list created when the free list is completed.
    timestamps: Vec<TimeValue>,
    /// Next object index to place or take an object from.
    object_index: usize,
    /// Array of addresses to the objects.
    objects: [*mut T; SIZE],
}

impl<T, const SIZE: usize> FreeList<T, SIZE> {
    /// Create a new free list that points to `next` free list.
    pub fn new(next: Option<Box<FreeList<T, SIZE>>>) -> Self {
        Self {
            next,
            timestamps: Vec::new(),
            object_index: 0,
            objects: [null_mut(); SIZE],
        }
    }

    /// Timestamp free list at current time for later comparison.
    pub fn make_timestamp(&mut self) {
        Self::save_current_timestamps(&mut self.timestamps);
    }

    /// Are all the timestamps of `&self` newer than the timestamps of `other`?
    pub fn is_newer_than(&self, other: &Self) -> bool {
        // compare each timestamp
        for (new, old) in self.timestamps.iter().zip(other.timestamps.iter()) {
            if !new.is_newer(old) {
                // one newer timestamp is older
                return false;
            }
        }
        // all are newer
        true
    }

    /// Are the objects owned by `&self` still in use?
    /// Uses `timestamps` as a buffer for current timestamps.
    pub fn is_in_use(&self, timestamps: &mut Vec<TimeValue>) -> bool {
        // update current timestamps
        Self::save_current_timestamps(timestamps);
        // compare each timestamp
        for (last, current) in self.timestamps.iter().zip(timestamps.iter()) {
            if !current.is_newer(last) {
                // if time has not progressed since the last time, then objects are still in use
                return true;
            }
        }
        // objects are not used
        false
    }

    /// Find the tail of the linked list of free lists starting from `self`.
    pub fn tail(&mut self) -> &mut Self {
        let mut current = self;
        while current.next.is_some() {
            // next free lists exist, continue
            current = current.next.as_deref_mut().unwrap();
        }
        // return current which is the tail
        current
    }

    // Save the current global timestamps to `timestamps` resizing the vector if needed.
    fn save_current_timestamps(timestamps: &mut Vec<TimeValue>) {
        // load current head of global thread time stamp list before resizing,
        // ensuring that the timestamp array always have space for each timestamp
        let mut current = GLOBAL_TIMESTAMPS
            .load(atomic::Ordering::Acquire)
            .cast_const();
        // reserve space for timestamps of every thread, if not available
        let len = GLOBAL_NEXT_ID.load(atomic::Ordering::Acquire);
        timestamps.resize(len, TimeValue::new(0));
        // iterate over each thread copying their time stamps
        while !current.is_null() {
            let timestamp = unsafe { current.read() };
            timestamps[timestamp.id] = timestamp.version;
            current = timestamp.next;
        }
    }
}

impl<T, const SIZE: usize> Drop for FreeList<T, SIZE> {
    fn drop(&mut self) {
        // drop the non-null pointers that are owned by the free list
        for obj in self.objects {
            if !obj.is_null() {
                unsafe {
                    drop(Box::from_raw(obj));
                }
            }
        }
        // also drop the tail of the free list iteratively in order to prevent
        // stack overflows that result from the default Drop recursion in Rust
        let mut next = self.next.take();
        while let Some(mut current) = next {
            next = current.next.take();
            // the following Drop call will have a None as the tail of current
            // stopping further recursions from occurring for current
            drop(current);
        }
        // head itself is dropped here
    }
}
