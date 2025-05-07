use std::{ptr::null_mut, sync::atomic::{AtomicBool, AtomicPtr as RawAtomicPtr, AtomicU64, AtomicUsize, Ordering}};
use std::sync::atomic::Ordering::SeqCst as SeqCst;
use haphazard::{AtomicPtr as HpAtomicPtr, HazardPointer};
use log::{debug, trace};

use crate::traits::{ConcurrentQueue, Handle};
use crossbeam::utils::CachePadded;

static RING_SIZE: u64 = 1024;
// static MAX_THREADS: usize = 256;

thread_local! {
    static THREAD_ID: std::cell::Cell<Option<usize>> = const {std::cell::Cell::new(None)};
}


pub struct LPRQueue<E> {
    head: CachePadded<HpAtomicPtr<PRQ<E>>>,
    tail: CachePadded<HpAtomicPtr<PRQ<E>>>,
    next_thread_id: AtomicUsize,
}

fn is_bottom<T>(value: *const T) -> bool {
    (value as usize & 1) != 0
}

fn thread_local_bottom<T>(tid: usize) -> *mut T {
    ((tid << 1) | 1) as *mut T
}    

impl<E: std::fmt::Debug> LPRQueue<E> {
    fn new() -> Self {
        let start = Box::into_raw(Box::new(PRQ::new()));
        LPRQueue {
            head: unsafe { CachePadded::new(HpAtomicPtr::new(start)) },
            tail: unsafe { CachePadded::new(HpAtomicPtr::new(start)) },
            next_thread_id: AtomicUsize::new(1),
        }
    }
    fn enqueue(&self, item: E, hp: &mut HazardPointer) {
        // trace!("Starting LPRQ enqueue");
        let mut inner_item = Box::into_raw(Box::new(item));
        let thread_id = self.get_thread_id();
        loop {
            let prq = self.tail.safe_load(hp).unwrap();
            // trace!("Enqueueing item now");
            match prq.enqueue(inner_item, thread_id) {
                Ok(()) => return,
                Err(val) => inner_item = val,
            }
            // trace!("Enqueue failed. PRQ is full.");
            let new_tail_ptr = Box::into_raw(Box::new(PRQ::new()));
            let new_tail = unsafe { new_tail_ptr.as_ref().unwrap() }; 
            // trace!("trying new enqueue, value of item is: {:?}", unsafe { inner_item.as_ref() });
            let _ = new_tail.enqueue(inner_item, thread_id);
            if unsafe { prq.next.compare_exchange_ptr(null_mut(), new_tail_ptr).is_ok() } {
                
                // trace!("switched next pointer to new tail");
                match unsafe { self.tail.compare_exchange_ptr(prq as *const _ as *mut _, new_tail_ptr) } {
                    Ok(_) => trace!("tail swap success"),
                    Err(_) => trace!("tail swap failure"),
                }
                return;
            } else {
                unsafe { drop(Box::from_raw(new_tail_ptr)); }
                let _ = unsafe { self.tail.compare_exchange_ptr(prq as *const _ as *mut _, prq.next.load_ptr()) };
            }
        }
    }
    fn dequeue(&self, hp: &mut HazardPointer) -> Option<E> {
        let thread_id = self.get_thread_id();
        loop {
            // trace!("Thread {thread_id}: starting lprqueue dequeue");
            let prq = self.head.safe_load(hp).unwrap();
            // trace!("Thread {thread_id}: Starting inner dequeue now");
            let mut res = prq.dequeue(thread_id);
            if res.is_some() {
                // trace!("Thread {thread_id}: Dequeue was a success");
                return res;
            }
            // trace!("Thread {thread_id}: Dequeue failed");
            if prq.next.load_ptr().is_null() {
                // self.trace_through();
                // trace!("Thread: {thread_id}: Returning none");
                return None;
            }
            res = prq.dequeue(thread_id);
            if res.is_some() {
                return res;
            }
            // trace!("Thread {thread_id}: prq is empty, update HEAD and restart");
            if let Ok(curr) = unsafe { self.head.compare_exchange_ptr(prq as *const _ as *mut _, prq.next.load_ptr()) } {
                let old_ptr = curr.unwrap();
                // self.crq_count.fetch_sub(1, Ordering::Relaxed);
                unsafe {
                    old_ptr.retire(); 
                }
            }
            hp.reset_protection();
        }
    }
    fn get_thread_id(&self) -> usize {
        THREAD_ID.with(|id| {
            if let Some(tid) = id.get() {
                // debug!("Got id {tid}");
                tid
            } else {
                let new_id = self.next_thread_id.fetch_add(1, Ordering::Relaxed);
                id.set(Some(new_id));
                debug!("Got id {new_id}");
                new_id
            }
        })
    }
}


#[derive(std::fmt::Debug)]
struct Cell<E> {
    safe_and_epoch: AtomicU64,
    value: RawAtomicPtr<E>,
}

impl<E> Cell<E> {
    fn new() -> Self {
        Self {
            safe_and_epoch: AtomicU64::new(1),
            value: RawAtomicPtr::new(null_mut()),
        }
    }
    fn safe_and_epoch(&self) -> (u64, bool, u64) {
        let safe_and_epoch = self.safe_and_epoch.load(Ordering::SeqCst);
        (safe_and_epoch, (safe_and_epoch & 1) == 1, safe_and_epoch >> 1)
    }
}

#[allow(non_snake_case)]
#[allow(clippy::upper_case_acronyms)]
struct PRQ<E> {
    next:CachePadded<HpAtomicPtr<PRQ<E>>>,
    closed:CachePadded<AtomicBool>,
    head:CachePadded<AtomicU64>,
    tail:CachePadded<AtomicU64>,
    A: Vec<Cell<E>>,
}

impl<E: std::fmt::Debug> PRQ<E> {
    fn new() -> Self {
        let mut a = Vec::with_capacity(RING_SIZE as usize);
        for _ in 0..RING_SIZE {
            a.push(Cell::new()); 
        }
        PRQ {
            head: CachePadded::new(AtomicU64::new(RING_SIZE)),
            tail: CachePadded::new(AtomicU64::new(RING_SIZE)),
            closed: CachePadded::new(AtomicBool::new(false)),
            next: CachePadded::new(unsafe { HpAtomicPtr::new(null_mut()) }),
            A: a,
        }
    }
    fn enqueue(&self, item: *mut E, thread_id: usize) -> Result<(), *mut E>{
        let item_ptr = item;
        loop {
            let t = self.tail.fetch_add(1, Ordering::SeqCst);
            if self.closed.load(Ordering::SeqCst) { 
                return Err(item);
            }
            let cycle: u64 = t / RING_SIZE;
            let i: usize = (t % RING_SIZE) as usize;
            
            let (whole, safe, epoch) = self.A[i].safe_and_epoch();
            let value = self.A[i].value.load(Ordering::SeqCst);
            // trace!("Thread {thread_id}: {safe}, {epoch}");
            // trace!("Thread {thread_id}: Checking if is_empty");
            let is_empty = value.is_null();
            // trace!("Thread {thread_id}: Checking if is_t");
            let is_t = !is_empty && is_bottom(value);
            let new_val = thread_local_bottom(thread_id);
            if (is_empty || is_t) &&
                epoch < cycle && (safe || self.head.load(Ordering::SeqCst) <= t)
            {
                // trace!("Thread {thread_id}: not occupied not overtaken");
                let cas_1 = self.A[i]
                    .value
                    .compare_exchange(
                        value as *const _ as *mut _,
                        new_val,
                        Ordering::SeqCst,
                        Ordering::SeqCst 
                        );
                if cas_1.is_err() {

                    // NOTE: CheckOverflow:
                    if t.wrapping_sub(self.head.load(SeqCst)) >= RING_SIZE {
                        self.closed.store(true, SeqCst);
                        return Err(item);
                    } else {
                        continue;
                    }
                } 
                let new_safe_and_epoch = (cycle << 1) | 1;
                if self.A[i].safe_and_epoch
                    .compare_exchange(
                        whole,
                        new_safe_and_epoch,
                        Ordering::SeqCst,
                        Ordering::SeqCst
                        ).is_err() {
                    // NOTE: Verify that this is allowed.
                    // trace!("Thread {thread_id}: Failed CAS 2");
                    // trace!("Thread {thread_id}: value is not null");
                    let _ =  self.A[i].value.compare_exchange(
                        new_val,
                        null_mut(),
                        SeqCst,
                        SeqCst);
                    // NOTE: CheckOverflow:
                    if t.wrapping_sub(self.head.load(SeqCst)) >= RING_SIZE {
                        self.closed.store(true, SeqCst);
                        return Err(item);
                    } else {
                        continue;
                    }
                }
                // trace!("Thread {thread_id}: Attempting to return item");
                // trace!("{:?}", *value);
                        // trace!("Thread {thread_id}: Managed to deref val");
                        // trace!("Thread {thread_id}: token: {token}, thread_id: {thread_id}");
                        // trace!("Thread {thread_id}: value: {:?}, self.value: {:?}", value, self.A[i].value);
                if self.A[i].value.compare_exchange(
                        new_val, 
                        item_ptr,
                        SeqCst,
                        SeqCst).is_ok() 
                {
                    return Ok(());
                } 
                // trace!("Thread {thread_id}: Failed to return item");
            }
            // NOTE: CheckOverflow:
            if t.wrapping_sub(self.head.load(SeqCst)) >= RING_SIZE {
                self.closed.store(true, SeqCst);
                return Err(item);
            } else {
                continue;
            }
        }
    } 
    fn dequeue(&self, _thread_id: usize) -> Option<E> {
        loop {
            let h = self.head.fetch_add(1, SeqCst);
            let cycle = h / RING_SIZE;
            let i = (h % RING_SIZE) as usize;
            loop {
                let (whole, safe, epoch) = self.A[i].safe_and_epoch();
                let value_ptr = self.A[i].value.load(SeqCst);
                let value = value_ptr;
                // Check if incosisten view of the cell
                if (whole, safe, epoch) != self.A[i].safe_and_epoch() {
                    continue;
                }
                // Is cell empty?
                let is_empty = value.is_null();
                // Is cell thread_id?
                let is_t = !is_empty && is_bottom(value);

                if epoch == cycle && (!is_empty && !is_t) {
                    let boxs = unsafe {
                        Box::from_raw(value_ptr)
                    };
                    let r_val = Some(*boxs);
                    self.A[i].value.store(null_mut(), SeqCst);
                    return r_val;
                } else if epoch <= cycle && (is_empty || is_t) {
                    if is_t 
                        && self.A[i].value.compare_exchange(value_ptr, null_mut(), SeqCst, SeqCst).is_err()
                    {
                        continue;
                    }
                    let new_safe_and_epoch = (cycle << 1) | (safe as u64);
                    if self.A[i].safe_and_epoch.compare_exchange(whole, new_safe_and_epoch, SeqCst, SeqCst).is_ok() {
                        break;
                    }
                } else if epoch < cycle && !(is_empty || is_t) {
                    // trace!("Thread {thread_id}: In case 3");
                    let new_safe_and_epoch = (epoch << 1) | (false as u64); // BUG: Was cycle here
                    if self.A[i].safe_and_epoch.compare_exchange(whole, new_safe_and_epoch, SeqCst, SeqCst).is_ok() {
                        break;
                    }
                } else {
                    // trace!("Thread {thread_id}: In case 4");
                    break;
                }
            }
            if self.tail.load(SeqCst) <= h + 1 { 
                // trace!("Thread {thread_id}: queue empty");
                self.fix_state();
                return None;
            }
        }
    } 
    fn fix_state(&self) {
        loop {
            let h = self.head.fetch_add(0, SeqCst);
            let t = self.tail.fetch_add(0, SeqCst);

            if self.tail.load(SeqCst) != t {continue;}
            if h < t {return}
            if self.tail.compare_exchange(t, h, SeqCst, SeqCst).is_ok() {return}
        }
    }
}


impl<T> Drop for LPRQueue<T> {
    fn drop(&mut self) {
        trace!("Starting drop LCRQueue");
        let head = unsafe {
            Box::from_raw(self.head.load_ptr())
        };
        let mut next = head.next;
        unsafe {

            while !next.load_ptr().is_null(){
                let node = Box::from_raw(next.load_ptr());
                trace!("Dropping CRQ");
                next = node.next;
            }
        }
        trace!("Done dropping");
    }
}

impl<T: std::fmt::Debug> ConcurrentQueue<T> for LPRQueue<T> {
    fn get_id(&self) -> String {
        "lprq_rust".to_string()
    }
    fn new(_size: usize) -> Self {
        LPRQueue::new()
    }
    fn register(&self) -> impl Handle<T> {
        LPRQueueHandle {
            queue: self,
            hp: HazardPointer::new(),
        } 
    }
}

struct LPRQueueHandle<'a, T> {
    queue: &'a LPRQueue<T>,
    hp: HazardPointer<'static>,
}

impl<T: std::fmt::Debug> Handle<T> for LPRQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        self.queue.dequeue(&mut self.hp)
    }
    fn push(&mut self, item: T) -> Result<(), T> {
        self.queue.enqueue(item, &mut self.hp);
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lprqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        let mut hp = HazardPointer::new();
        q.enqueue(1, &mut hp);
        assert_eq!(q.dequeue(&mut hp).unwrap(), 1);
    }
    #[test]
    fn register_lprqueue() {
        let q: LPRQueue<i32> = LPRQueue::new();
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
    #[test]
    fn enqueue_full_prq() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        let mut hp = HazardPointer::new();
        for _ in 0..RING_SIZE + 3 {
            q.enqueue(1, &mut hp);
        }
        for _ in 0..RING_SIZE + 3 {
            assert_eq!(q.dequeue(&mut hp).unwrap(), 1);
        }
        
    }
    #[test]
    fn enqueue_full_prq2() {

        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        let mut hp = HazardPointer::new();
        for _ in 0..RING_SIZE + 3 {
            q.enqueue(1, &mut hp);
        }
        for _ in 0..RING_SIZE + 3 {
            assert_eq!(q.dequeue(&mut hp).unwrap(), 1);
        }
    }
    #[test]
    fn multi_thread() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<usize> = LPRQueue::new();
        let thread_count = 50;
        let barrier = std::sync::Barrier::new(thread_count);
        let sum = AtomicUsize::new(0);
        std::thread::scope(|s| {
            let q = &q;      
            let barrier = &barrier;
            let sum = &sum;
            for _ in 0..thread_count {
                s.spawn(move|| {
                    let mut handle = q.register();
                    barrier.wait();
                    let mut local = 0;
                    for i in 0..thread_count * 10 {
                        let _ = handle.push(i + 1);
                        local += i + 1;
                    } 
                    sum.fetch_add(local, SeqCst);
                });
            }
        });
        let mut thesum = 0;
        let mut h = q.register();
        while let Some(val) = h.pop() {
            thesum += val;
        }
        assert_eq!(thesum, sum.into_inner());
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        if crate::order::benchmark_order_i32(q, 50, 10, true, 10).is_err() {
            panic!();
        }
    }
}
