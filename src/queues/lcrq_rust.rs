use std::arch::asm;
#[allow(unused_imports)]
use std::{mem::MaybeUninit, ptr::null_mut, sync::atomic::{AtomicBool, AtomicPtr as RawAtomicPtr, AtomicU64, AtomicUsize, Ordering}};
#[allow(unused_imports)]
use std::sync::atomic::Ordering::SeqCst as SeqCst;
#[allow(unused_imports)]
use haphazard::{raw::Pointer, AtomicPtr as HpAtomicPtr, HazardPointer};
#[allow(unused_imports)]
use log::{debug, error, trace};
#[allow(unused_imports)]
use crate::traits::{ConcurrentQueue, Handle};
use crossbeam::utils::CachePadded;

static RING_SIZE: usize = 1024;
// static MAX_THREADS: usize = 256;

pub struct LCRQueue<T: std::fmt::Debug> {
    tail: CachePadded<HpAtomicPtr<CRQ<T>>>,
    head: CachePadded<HpAtomicPtr<CRQ<T>>>,
    // crq_count: AtomicU64,
}

impl<T: std::fmt::Debug> Drop for LCRQueue<T> {
    fn drop(&mut self) {
        trace!("Starting drop LCRQueue");
        let head = unsafe {
            Box::from_raw(self.head.load_ptr())
        };
        let mut next = head.next;
        // debug!("{:?}", self.crq_count);
        unsafe {

            while !next.load_ptr().is_null(){
                let node = Box::from_raw(next.load_ptr());
                trace!("Dropping CRQ");
                next = node.next;
            }
        }
        trace!("Done dropping");
        // let mut hp = HazardPointer::new();
        // while self.dequeue(&mut hp).is_some() {
        //
        // }
        // unsafe {
        //     drop(Box::from_raw(self.head.load_ptr()));
        // }
    }
}

fn to_mut_ptr<T>(item: &T) -> *mut T {
    item as *const T as *mut T
}
impl<T: std::fmt::Debug> LCRQueue<T> {
    fn new() -> Self {
        let ptr = Box::into_raw(Box::new(CRQ::new()));
        LCRQueue {
            tail: unsafe { CachePadded::new(HpAtomicPtr::new(ptr)) },
            head: unsafe { CachePadded::new(HpAtomicPtr::new(ptr)) },
            // crq_count: AtomicU64::new(1),
        }
    }
    #[allow(dead_code)]
    fn trace_through(&self) {
        trace!("############ STARTING TRACE THROUGH ######################");
        let mut curr = unsafe { self.head.load_ptr().as_ref().unwrap() };
        loop {
            for cell in &curr.ring {
                if cell.value.load(SeqCst).is_null() {
                    trace!("null");
                } else {
                    unsafe {trace!("{:?}", *cell.value.load(SeqCst))}
                }
            }
            trace!("##### NEW PRQ #####");
            let tmp  = unsafe { curr.next.load_ptr().as_ref() };
            curr = if let Some(val) = tmp {
                val 
            } else {
                break;
            }
        }
        trace!("############## ENDING TRACE THROUGH ######################");
    }
    fn dequeue(&self, hp: &mut HazardPointer) -> Option<T> {
        trace!("Starting outer dequeue now");
        // self.trace_through();
        loop {
            let crq = self.head.safe_load(hp).unwrap();
            let v = crq.dequeue();
            if v.is_some() {
                // trace!("{:?}: Got the item", std::thread::current().id());
                return v;
            }
            let crq_next = crq.next.load_ptr();
            if crq_next.is_null() {
                hp.reset_protection();
                return None;
            }
            if let Ok(curr) = unsafe { self.head.compare_exchange_ptr(to_mut_ptr(crq), crq_next) } {
                let old_ptr = curr.unwrap();
                // self.crq_count.fetch_sub(1, Ordering::Relaxed);
                unsafe {
                    old_ptr.retire(); 
                }
            }
            hp.reset_protection();
        } 
    }
    fn enqueue(&self, item: T, hp: &mut HazardPointer) {
        trace!("Starting LCRQ enqueue");
        let mut inner_item = Box::into_raw(Box::new(item));
        loop {
            let crq = self.tail.safe_load(hp).unwrap();
            // let crq = unsafe { crq_ptr.as_ref().unwrap() };
            trace!("Enqueueing item now");
            match crq.enqueue(inner_item) {
                Ok(()) => return,
                Err(val) => inner_item = val,
            }
            trace!("Enqueue failed. CRQ is full.");
            let new_tail_ptr = Box::into_raw(Box::new(CRQ::new()));
            let new_tail = unsafe { new_tail_ptr.as_ref().unwrap() }; 
            new_tail.ring[0].value.store(inner_item, Ordering::Relaxed);
            new_tail.tail.store(1, Ordering::Relaxed);
            new_tail.ring[0].safe_and_idx.store(0, Ordering::Relaxed);
            // trace!("trying new enqueue, value of item is: {:?}", unsafe { inner_item.as_ref() });
            // let _ = new_tail.enqueue(inner_item);
            unsafe {
                if crq.next.compare_exchange_ptr(null_mut(), new_tail_ptr).is_ok() {
                    trace!("switched next pointer to new tail");
                    // self.crq_count.fetch_add(1, Ordering::Relaxed);
                    // What does this failing mean? Another thread already helped?
                    match self.tail.compare_exchange_ptr(to_mut_ptr(crq), new_tail_ptr) {
                        Ok(_old_ptr) => {
                            trace!("tail swap success");
                        },
                        Err(_) => trace!("tail swap failure"),
                    }
                    hp.reset_protection();
                    return;
                } else {
                    trace!("failed to insert new");
                    let reclaimed_new = Box::from_raw(new_tail_ptr);
                    inner_item = Box::into_raw(Box::new(reclaimed_new.dequeue().unwrap()));
                    drop(reclaimed_new);
                    // Help other thread
                    let _ = self.tail.compare_exchange_ptr(to_mut_ptr(crq), crq.next.load_ptr());
                }
            }
        }
    }
}

#[derive(std::fmt::Debug)]
#[repr(C, align(16))]
struct Cell<E: std::fmt::Debug> {
    safe_and_idx: AtomicU64,
    value: RawAtomicPtr<E>,
}

impl<E: std::fmt::Debug> Cell<E> {
    fn new() -> Self {
        Self {
            safe_and_idx: AtomicU64::new(1),
            value: RawAtomicPtr::new(null_mut()),
        }
    }
    fn safe_and_idx(&self) -> (bool, u64) {
        let safe_and_epoch = self.safe_and_idx.load(Ordering::SeqCst);
        ((safe_and_epoch & 1) == 1, safe_and_epoch >> 1)
    }
}

impl<E: std::fmt::Debug> Drop for Cell<E> {
    fn drop(&mut self) {
        unsafe {
            trace!("Dropping Cell now.");
            let ptr: *mut E = self.value.load(Ordering::SeqCst);
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(std::fmt::Debug)]
struct CRQ<T: std::fmt::Debug> {
    head: CachePadded<AtomicU64>,
    tail: CachePadded<AtomicU64>,
    closed: CachePadded<AtomicBool>,
    next: CachePadded<HpAtomicPtr<CRQ<T>>>,
    ring: Vec<Cell<T>>,
}

impl<T: std::fmt::Debug> CRQ<T> {
    fn new() -> Self {
        let mut ring = Vec::with_capacity(RING_SIZE);
        for _ in 0..RING_SIZE {
            ring.push(Cell::new());
        }
        CRQ {
            head: CachePadded::new(AtomicU64::new(0)),
            tail: CachePadded::new(AtomicU64::new(0)),
            closed: CachePadded::new(AtomicBool::new(false)),
            next: unsafe { CachePadded::new(HpAtomicPtr::new(null_mut())) },
            ring,
        }
    }
    fn dequeue(&self) -> Option<T> {
        trace!("Starting inner dequeue now");
        loop {
            let h = self.head.fetch_add(1, SeqCst);
            let node = &self.ring[h as usize % RING_SIZE];
            loop {
                let val = node.value.load(SeqCst);
                let (safe, idx) = node.safe_and_idx();
                if idx > h {
                    // Line 52
                    trace!("Inner dequeue: idx > h heading to line 52");
                    break;
                }
                #[allow(clippy::collapsible_if)]
                if !val.is_null() {
                    if idx == h {
                        trace!("Inner dequeue: idx == h");
                        // try dequeue
                        if cas2_w(node, create_safe_idx(safe, h), val, create_safe_idx(false, h + RING_SIZE as u64), null_mut()) {
                            unsafe {
                                let boxs = Box::from_raw(val);
                                return Some(*boxs);
                            }
                        } 
                    } else {
                        // mark node unsafe to prevent future enqueue
                        trace!("Inner dequeue: Marking node unsafe to prevent future enqueue");
                        if cas2_w(node, create_safe_idx(safe, h), val, create_safe_idx(false, h), val) {
                            // Line 52
                            break;
                        }
                    }
                } else { unsafe {
                    // idx <= h and val == empty; try empty transition
                    trace!("Inner dequeue: Trying empty transition");
                    // NOTE: This is optimisation 1 from the paper.
                    // Unsure if this is how they meant. Could not get this
                    // to perform better than without.
                    // let tail = self.tail.load(SeqCst);
                    // if tail > h {
                    //     for _ in 0..10 {
                    //         std::hint::spin_loop();
                    //     }
                    // }

                    if cas2_w(node, create_safe_idx(safe, idx), val, create_safe_idx(safe, h + RING_SIZE as u64), null_mut()) {
                        // // Line 52
                        // println!("{:?}: cas2 success", std::thread::current().id());
                        drop(Box::from_raw(val));
                        break;
                    }
                } }

            }
            // Line 52
            let tail = self.tail.load(SeqCst);
            // let closed = tail_and_closed & 1 == 1;
            if tail <= h + 1 {
                trace!("Inner dequeue: Fixing state and returning");
                self.fix_state();
                return None;
            }
        }
    }
    fn enqueue(&self, item: *mut T) -> Result<(), *mut T>{
        trace!("Starting inner enqueue now");
        loop {
            let t = self.tail.fetch_add(1, SeqCst);
            let closed = self.closed.load(SeqCst);
            if closed {
                trace!("Inner enqueue: CRQ closed.");
                // unsafe {
                //     if let CellValue::Value(ref item_val) = *Box::from_raw(item) {
                //         return Err(std::ptr::read(item_val.assume_init_ref() as *const _));
                //     }
                // }
                return Err(item);
            }
            let index = t as usize % RING_SIZE;
            trace!("Inner enqueue: index: {index}");
            let node = &self.ring[t as usize % RING_SIZE];
            let val = node.value.load(SeqCst);
            let (safe, idx) = node.safe_and_idx();
            if val.is_null() {
                trace!("Inner enqueue: val was empty");
                trace!("Inner enqueue: idx:{idx} t:{t}");
                if idx <= t &&
                   (safe || self.head.load(SeqCst) <= t) &&
                   cas2_w(node, create_safe_idx(safe, idx), val, create_safe_idx(true, t), item) {
                    return Ok(());
                }
            }
            let h = self.head.load(SeqCst);
            if t >= h && t - h >= RING_SIZE as u64 {
                self.closed.store(true, SeqCst);
                // unsafe {
                //     if let CellValue::Value(ref item_val) = *Box::from_raw(item) {
                //         return Err(std::ptr::read(item_val.assume_init_ref() as *const _));
                //     }
                // }
                return Err(item);
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



fn create_safe_idx(safe: bool, idx: u64) -> u64 {
    (idx << 1) | safe as u64
}

fn cas2_w<T: std::fmt::Debug>(
    node: &Cell<T>,
    safe_and_idx: u64,
    val: *mut T,
    new_safe_and_idx: u64,
    new_val: *mut T
) -> bool {
    let ptr = node as *const Cell<T> as *const u128 as *mut u128;
    let expected_low = safe_and_idx;
    let expected_high = val as *const u64 as *mut u64;
    let new_low = new_safe_and_idx;
    let new_high = new_val as *const u64 as *mut u64;
    // println!("{:?}: starting cas2", std::thread::current().id());
    cas2(ptr, expected_low, expected_high, new_low, new_high)
}

#[inline]
pub fn cas2(
    ptr: *mut u128,
    expected_low: u64,
    expected_high: *mut u64,
    new_low: u64,
    new_high: *mut u64,
) -> bool {
    assert_eq!(ptr as usize & 0xF, 0);
    let result: u8;
    unsafe {
        asm!(
            "push rbx",
            "mov rbx, {new_low}",
            "lock cmpxchg16b [{ptr}]",
            "setz {result}",
            "pop rbx",
            ptr = in(reg) ptr,
            result = out(reg_byte) result,
            new_low = in(reg) new_low,
            inout("rax") expected_low => _,
            inout("rdx") expected_high => _,
            in("rcx") new_high,
            options(preserves_flags)
        );
    }
    result != 0
}

impl<T: std::fmt::Debug> ConcurrentQueue<T> for LCRQueue<T> {
    fn get_id(&self) -> String {
        "lcrq_rust".to_string()
    }
    fn new(_size: usize) -> Self {
        LCRQueue::new()
    }
    fn register(&self) -> impl Handle<T> {
        LCRQueueHandle {
            queue: self,
            hp1: HazardPointer::new(),
            // hp2: HazardPointer::new(),
        } 
    }
}

struct LCRQueueHandle<'a, T: std::fmt::Debug> {
    queue: &'a LCRQueue<T>,
    hp1: HazardPointer<'static>,
    // hp2: HazardPointer<'static>, 
}

impl<T: std::fmt::Debug> Handle<T> for LCRQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        self.queue.dequeue(&mut self.hp1)
    }
    fn push(&mut self, item: T) -> Result<(), T> {
        self.queue.enqueue(item, &mut self.hp1);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicI32;

    use log::info;

    use super::*;

    #[test]
    fn create_lcrqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        info!("starting test");
        let q: LCRQueue<i32> = LCRQueue::new();
        let mut hp = HazardPointer::new();
        q.enqueue(1, &mut hp);
        assert_eq!(q.dequeue(&mut hp).unwrap(), 1);
    }
    #[test]
    fn register_lcrqueue() {
        let q: LCRQueue<i32> = LCRQueue::new();
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
    #[test]
    fn enqueue_full_crq() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LCRQueue<i32> = LCRQueue::new();
        let mut hp = HazardPointer::new();
        for _ in 0..RING_SIZE + 3 {
            q.enqueue(1, &mut hp);
        }
        for _ in 0..RING_SIZE + 3 {
            assert_eq!(q.dequeue(&mut hp).unwrap(), 1);
        }
        
    }
    #[test]
    fn drop_test() {

        let _ = env_logger::builder().is_test(true).try_init();
        let q: LCRQueue<i32> = LCRQueue::new();
        let mut hp = HazardPointer::new();
        info!("Starting enqueues");
        for i in 0..RING_SIZE * 2 {
            q.enqueue(i as i32, &mut hp);
        }
        info!("Starting dequeues");
        for _ in 0..RING_SIZE + 1 {
            let val = q.dequeue(&mut hp).unwrap();
            debug!("Value was {val}");
        }
        info!("Test done");
    }
    #[test]
    fn enqueue_full2_crq() {

        let _ = env_logger::builder().is_test(true).try_init();
        let q: LCRQueue<i32> = LCRQueue::new();
        let mut curr = 0;
        let mut hp = HazardPointer::new();
        for _ in 0..RING_SIZE + 3 {
            q.enqueue(curr, &mut hp);
            curr += 1;
        }
        curr = 0;
        for _ in 0..RING_SIZE + 3 {
            assert_eq!(q.dequeue(&mut hp).unwrap(), curr);
            curr += 1;
        }
    }
    #[test]
    fn multi_thread() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LCRQueue<i32> = LCRQueue::new();
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
        let mut handle = q.register();
        while let Some(val) = handle.pop() {
            thesum += val;
        }
        assert_eq!(thesum, sum.into_inner());
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LCRQueue<i32> = LCRQueue::new();
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
