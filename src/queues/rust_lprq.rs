use std::{mem::MaybeUninit, ptr::null_mut, sync::atomic::{AtomicBool, AtomicPtr as RawAtomicPtr, AtomicU64, AtomicUsize, Ordering}};
use std::sync::atomic::Ordering::SeqCst as SeqCst;
// use haphazard::{raw::Pointer, AtomicPtr as HpAtomicPtr, HazardPointer};
use log::{error, trace};

use crate::traits::{ConcurrentQueue, Handle};

static RING_SIZE: u64 = 1024;
// static MAX_THREADS: usize = 256;

thread_local! {
    static THREAD_ID: std::cell::Cell<Option<usize>> = const {std::cell::Cell::new(None)};
}


pub struct LPRQueue<E> {
    head: RawAtomicPtr<PRQ<E>>,
    tail: RawAtomicPtr<PRQ<E>>,
    next_thread_id: AtomicUsize,
}

impl<E: std::fmt::Debug> LPRQueue<E> {
    fn trace_through(&self) {
        let mut curr = unsafe { self.head.load(SeqCst).as_ref().unwrap() };
        loop {
            for cell in &curr.A {
                if cell.value.load(SeqCst).is_null() {
                    trace!("null");
                } else {
                    unsafe {trace!("{:?}", *cell.value.load(SeqCst))}
                }
            }
            let tmp  = unsafe { curr.next.load(SeqCst).as_ref() };
            curr = if let Some(val) = tmp {
                val 
            } else {
                break;
            }
        }
    }
    fn new() -> Self {
        let start = Box::into_raw(Box::new(PRQ::new()));
        LPRQueue {
            head: RawAtomicPtr::new(start),
            tail: RawAtomicPtr::new(start), 
            next_thread_id: AtomicUsize::new(1),
        }
    }
    fn enqueue(&self, item: E) {
        trace!("Starting LPRQ enqueue");
        let mut inner_item = Box::into_raw(Box::new(CellValue::Value(MaybeUninit::new(item))));
        loop {
            let prq_ptr = self.tail.load(SeqCst);
            let prq = unsafe { prq_ptr.as_ref().unwrap() };
            trace!("Enqueueing item now");
            match prq.enqueue(inner_item, self.get_thread_id()) {
                Ok(()) => return,
                Err(val) => inner_item = Box::into_raw(Box::new(CellValue::Value(MaybeUninit::new(val)))),
            }
            trace!("Enqueue failed. PRQ is full.");
            let new_tail_ptr = Box::into_raw(Box::new(PRQ::new()));
            let new_tail = unsafe { new_tail_ptr.as_ref().unwrap() }; 
            trace!("trying new enqueue, value of item is: {:?}", unsafe { inner_item.as_ref() });
            let _ = new_tail.enqueue(inner_item, self.get_thread_id());
            if prq.next.compare_exchange(null_mut(), new_tail_ptr, SeqCst, SeqCst).is_ok() {
                
                trace!("switched next pointer to new tail");
                match self.tail.compare_exchange(prq_ptr, new_tail_ptr, SeqCst, SeqCst) {
                    Ok(_) => trace!("tail swap success"),
                    Err(_) => trace!("tail swap failure"),
                }
                return;
            } else {
                let _ = self.tail.compare_exchange(prq_ptr, prq.next.load(SeqCst), SeqCst, SeqCst);
            }
        }
    }
    fn dequeue(&self) -> Option<E> {
        loop {
            trace!("starting lprqueue dequeue");
            let prq_ptr = self.head.load(SeqCst);
            let prq = unsafe { prq_ptr.as_ref().unwrap() };
            trace!("Starting inner dequeue now");
            let mut res = prq.dequeue();
            if res.is_some() {
                trace!("Dequeue was a success");
                return res;
            }
            trace!("Dequeue failed");
            if prq.next.load(SeqCst).is_null() {

                return None;
            }
            res = prq.dequeue();
            if res.is_some() {
                return res;
            }
            trace!("prq is empty, update HEAD and restart");
            // NOTE: Drop old one here?
            // self.trace_through();
            let _ = self.head.compare_exchange(prq_ptr, prq.next.load(SeqCst), SeqCst, SeqCst);
        }
    }
    fn get_thread_id(&self) -> usize {
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
}

#[derive(std::fmt::Debug)]
enum CellValue<E> {
    Empty,
    ThreadToken(usize),
    Value(MaybeUninit<E>),
}


#[derive(std::fmt::Debug)]
struct Cell<E> {
    safe_and_epoch: AtomicU64,
    value: RawAtomicPtr<CellValue<E>>,
}

impl<E> Cell<E> {
    fn new() -> Self {
        Self {
            safe_and_epoch: AtomicU64::new(1),
            value: RawAtomicPtr::new(Box::into_raw(Box::new(CellValue::Empty))),
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
    next: RawAtomicPtr<PRQ<E>>,
    closed: AtomicBool,
    A: Vec<Cell<E>>,
    head: AtomicU64,
    tail: AtomicU64,
}

impl<E: std::fmt::Debug> PRQ<E> {
    fn new() -> Self {
        let mut a = Vec::with_capacity(RING_SIZE as usize);
        for _ in 0..RING_SIZE {
            a.push(Cell::new()); 
        }
        PRQ {
            head: AtomicU64::new(RING_SIZE),
            tail: AtomicU64::new(RING_SIZE),
            closed: AtomicBool::new(false),
            A: a,
            next: RawAtomicPtr::new(null_mut()),
        }
    }
    fn enqueue(&self, item: *mut CellValue<E>, thread_id: usize) -> Result<(), E>{
        let item_ptr = item;
        loop {
            // for cell in &self.A {
            //     if cell.value.load(SeqCst).is_null() {
            //         trace!("null");
            //     } else {
            //         unsafe {trace!("{:?}", *cell.value.load(SeqCst))}
            //     }
            // }
            let t = self.tail.fetch_add(1, Ordering::SeqCst);
            if self.closed.load(Ordering::SeqCst) { 
                if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                    return Err(unsafe{val.assume_init()});
                }
            }
            let cycle: u64 = t / RING_SIZE;
            let i: usize = (t % RING_SIZE) as usize;
            
            let (whole, safe, epoch) = self.A[i].safe_and_epoch();
            let mut value = self.A[i].value.load(Ordering::SeqCst);
            trace!("{safe}, {epoch}");
            trace!("Checking if is_empty");
            let is_empty = unsafe {
                if value.is_null() {
                    true
                } else {
                    matches!(*value, CellValue::Empty)
                }
            };
            trace!("Checking if is_t");
            let is_t = unsafe {
                if is_empty { 
                    false
                }
                else {matches!(*value, CellValue::ThreadToken(_))}
            };
            if (is_empty || is_t) &&
                epoch < cycle && (safe || self.head.load(Ordering::SeqCst) <= t)
            {
                trace!("not occupied not overtaken");
                let new_val = Box::into_raw(Box::new(CellValue::ThreadToken(thread_id)));
                let cas_1 = self.A[i]
                    .value
                    .compare_exchange(
                        value,
                        new_val,
                        Ordering::SeqCst,
                        Ordering::SeqCst 
                        );
                if cas_1.is_err() {
                    trace!("Failed CAS 1");
                    if check_overflow(t, self.head.load(SeqCst), &self.closed) {
                        continue;
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                            return Err(unsafe{val.assume_init()});
                        }
                    }
                } else {
                    value = new_val;
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
                    trace!("Failed CAS 2");
                    unsafe {
                        if !value.is_null() {
                            trace!("value is not null");
                            if let CellValue::ThreadToken(token) = *value {
                                if token == thread_id {
                                    let new_val = Box::into_raw(Box::new(CellValue::Empty));
                                    let _ =  self.A[i].value.compare_exchange(value, new_val, SeqCst, SeqCst);
                                } 
                            }
                        }
                    }
                    if unsafe {matches!(*value, CellValue::ThreadToken(_))} {
                        let _ = self.A[i].value.compare_exchange(value, new_val, SeqCst, SeqCst);
                    }
                    if check_overflow(t, self.head.load(SeqCst), &self.closed) {
                        continue;
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                            return Err(unsafe{val.assume_init()});
                        }
                    }
                }
                trace!("Attempting to return item");
                unsafe {
                    if !value.is_null() {
                        trace!("Value was not null");
                        trace!("{:?}", *value);
                        if let CellValue::ThreadToken(token) = *value {
                            trace!("Managed to deref val");
                            trace!("token: {token}, thread_id: {thread_id}");
                            trace!("value: {:?}, self.value: {:?}", value, self.A[i].value);
                            if token == thread_id 
                                && self.A[i].value.compare_exchange(value, item_ptr, SeqCst, SeqCst).is_ok() 
                            {
                                trace!("Managed to enqueue");
                                return Ok(());
                            } 
                        }
                    }
                }
                trace!("Failed to return item");
            }
            if !check_overflow(t, self.head.load(SeqCst), &self.closed) {
                trace!("PRQ full now");
                #[allow(clippy::collapsible_if)]
                if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                    trace!("item_ptr is still a value");
                    return Err(unsafe{val.assume_init()});
                }
            }
        }
    } 
    fn dequeue(&self) -> Option<E> {
        loop {
            // trace!("{:?}", self.A);
            // for cell in &self.A {
            //     if cell.value.load(SeqCst).is_null() {
            //         trace!("null");
            //     } else {
            //         unsafe {trace!("{:?}", *cell.value.load(SeqCst))}
            //     }
            // }
            let h = self.head.fetch_add(1, SeqCst);
            let cycle = h / RING_SIZE;
            let i = (h % RING_SIZE) as usize;
            loop {
                let (whole, safe, epoch) = self.A[i].safe_and_epoch();
                let value = self.A[i].value.load(SeqCst);
                if (whole, safe, epoch) != self.A[i].safe_and_epoch() {
                    continue;
                }
                let is_empty = unsafe {
                    if value.is_null() {
                        true
                    } else {
                        matches!(*value, CellValue::Empty)
                    }
                };
                let is_t = unsafe {
                    if is_empty { false }
                    else {matches!(*value, CellValue::ThreadToken(_))}
                };
                if epoch == cycle && (!is_empty && !is_t) {
                    trace!("In case 1");
                    let boxs = unsafe {
                        Box::from_raw(value)
                    };
                    trace!("{:?}", *boxs);
                    if let CellValue::Value(val) = *boxs{
                        let r_val = Some(unsafe {std::ptr::read(val.assume_init_ref())});
                        trace!("{:?}", r_val);
                        self.A[i].value.store(to_raw(CellValue::Empty), SeqCst);
                        return r_val;
                    } else {
                        error!("Cell contained no value.");
                    }
                } else if epoch <= cycle && (is_empty || is_t) {
                    trace!("In case 2");
                    if is_t 
                        && self.A[i].value.compare_exchange(value, to_raw(CellValue::Empty), SeqCst, SeqCst).is_err()
                    {
                        continue;
                    }
                    let new_safe_and_epoch = (cycle << 1) | (safe as u64);
                    if self.A[i].safe_and_epoch.compare_exchange(whole, new_safe_and_epoch, SeqCst, SeqCst).is_ok() {
                        break;
                    }
                } else if epoch < cycle && (!is_empty && !is_t) {
                    trace!("In case 3");
                    let new_safe_and_epoch = (cycle << 1) | (false as u64);
                    if self.A[i].safe_and_epoch.compare_exchange(whole, new_safe_and_epoch, SeqCst, SeqCst).is_ok() {
                        break;
                    }
                } else {
                    trace!("In case 4");
                    break;
                }
            }
            if self.tail.load(SeqCst) <= h + 1 { return None }
        }
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
        } 
    }
}

struct LPRQueueHandle<'a, T> {
    queue: &'a LPRQueue<T>,
}

impl<T: std::fmt::Debug> Handle<T> for LPRQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        self.queue.dequeue()
    }
    fn push(&mut self, item: T) -> Result<(), T> {
        self.queue.enqueue(item);
        Ok(())
    }
}

fn to_raw<T>(item: T) -> *mut T {
    Box::into_raw(Box::new(item))
}


fn check_overflow(t: u64, head: u64, closed: &AtomicBool) -> bool {
    // HACK: Check the t >= head part
    if t >= head && t - head >= RING_SIZE {
        closed.store(true, Ordering::SeqCst);
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicI32;

    use super::*;

    #[test]
    fn create_lprqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        q.enqueue(1);
        assert_eq!(q.dequeue().unwrap(), 1);
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
        for _ in 0..RING_SIZE + 3 {
            q.enqueue(1);
        }
        for _ in 0..RING_SIZE + 3 {
            assert_eq!(q.dequeue().unwrap(), 1);
        }
        
    }
    #[test]
    fn enqueue_full_prq2() {

        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        let mut curr = 0;
        for _ in 0..RING_SIZE + 3 {
            q.enqueue(curr);
            curr += 1;
        }
        curr = 0;
        for _ in 0..RING_SIZE + 3 {
            assert_eq!(q.dequeue().unwrap(), curr);
            curr += 1;
        }
    }
    #[test]
    fn multi_thread() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        let barrier = std::sync::Barrier::new(10);
        let sum = AtomicI32::new(0);
        std::thread::scope(|s| {
            let q = &q;      
            let barrier = &barrier;
            let sum = &sum;
            for _ in 0..10 {
                s.spawn(move|| {
                    let mut handle = q.register();
                    barrier.wait();
                    let mut local = 0;
                    for i in 0..10 {
                        let _ = handle.push(i + 1);
                        local += i + 1;
                    } 
                    sum.fetch_add(local, SeqCst);
                });
            }
        });
        let mut thesum: i32 = 0;
        while let Some(val) = q.dequeue() {
            thesum += val;
        }
        assert_eq!(thesum, sum.into_inner());
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LPRQueue<i32> = LPRQueue::new();
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
