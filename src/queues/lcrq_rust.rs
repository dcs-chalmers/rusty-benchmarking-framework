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

static RING_SIZE: usize = 4;
static MAX_THREADS: usize = 256;

pub struct LCRQueue<T: std::fmt::Debug> {
    tail: HpAtomicPtr<CRQ<T>>,
    head: HpAtomicPtr<CRQ<T>>,
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
                debug!("Dropping CRQ");
                next = node.next;
            }
        }
        trace!("Done dropping");
    }
}

fn to_mut_ptr<T>(item: &T) -> *mut T {
    item as *const T as *mut T
}
impl<T: std::fmt::Debug> LCRQueue<T> {
    fn new() -> Self {
        let ptr = Box::into_raw(Box::new(CRQ::new()));
        LCRQueue {
            tail: unsafe { HpAtomicPtr::new(ptr) },
            head: unsafe { HpAtomicPtr::new(ptr) },
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
            if crq_next.is_null() {return None;}
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
        let mut inner_item = Box::into_raw(Box::new(CellValue::Value(MaybeUninit::new(item))));
        loop {
            let crq = self.tail.safe_load(hp).unwrap();
            // let crq = unsafe { crq_ptr.as_ref().unwrap() };
            trace!("Enqueueing item now");
            match crq.enqueue(inner_item) {
                Ok(()) => return,
                Err(val) => inner_item = Box::into_raw(Box::new(CellValue::Value(MaybeUninit::new(val)))),
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
                    inner_item = Box::into_raw(Box::new(CellValue::Value(MaybeUninit::new(reclaimed_new.dequeue().unwrap()))));
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
struct Cell<E> {
    safe_and_idx: AtomicU64,
    value: RawAtomicPtr<CellValue<E>>,
}

impl<E> Cell<E> {
    fn new() -> Self {
        Self {
            safe_and_idx: AtomicU64::new(1),
            value: RawAtomicPtr::new(Box::into_raw(Box::new(CellValue::Empty))),
        }
    }
    fn safe_and_idx(&self) -> (bool, u64) {
        let safe_and_epoch = self.safe_and_idx.load(Ordering::SeqCst);
        ((safe_and_epoch & 1) == 1, safe_and_epoch >> 1)
    }
}

impl<E> Drop for Cell<E> {
    fn drop(&mut self) {
        unsafe {
            debug!("Dropping Cell now.");
            let ptr = self.value.load(Ordering::SeqCst);
            drop(Box::from_raw(ptr));
        }
    }
}

enum CellValue<E> {
    Empty,
    Value(MaybeUninit<E>),
}

impl<E> Drop for CellValue<E> {
    fn drop(&mut self) {
        debug!("Dropping CellValue now");
        if let CellValue::Value(val) = self {
            unsafe {
                // Take ownership of the value and drop it
                std::ptr::drop_in_place(val.as_mut_ptr());
            }
        }
    }
}

impl<E: std::fmt::Debug> std::fmt::Debug for CellValue<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellValue::Empty => write!(f, "Empty"),
            CellValue::Value(val) => write!(f, "Value: {:?}", unsafe { val.assume_init_ref() }),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(std::fmt::Debug)]
struct CRQ<T> {
    head: AtomicU64,
    tail: AtomicU64,
    closed: AtomicBool,
    next: HpAtomicPtr<CRQ<T>>,
    ring: Vec<Cell<T>>,
}

// impl<T> Drop for CRQ<T> {
//     fn drop(&mut self) {
//         debug!("Dropping CRQ now")
//     }
// }

impl<T: std::fmt::Debug> CRQ<T> {
    fn new() -> Self {
        let mut ring = Vec::with_capacity(RING_SIZE);
        for _ in 0..RING_SIZE {
            ring.push(Cell::new());
        }
        CRQ {
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            next: unsafe { HpAtomicPtr::new(null_mut()) },
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
                // let val_ref = unsafe { val.as_ref() }.unwrap();
                let val_ref = unsafe { match val.as_ref() {
                    Some(val_r) => val_r,
                    None => {
                        error!("Inner dequeue: Value was None.");
                        return None;
                    } ,
                }};
                let (safe, idx) = node.safe_and_idx();
                if idx > h {
                    // Line 52
                    trace!("Inner dequeue: idx > h heading to line 52");
                    break;
                }
                #[allow(clippy::collapsible_if)]
                if !(matches!(val_ref, CellValue::Empty)) {
                    if idx == h {
                        trace!("Inner dequeue: idx == h");
                        // try dequeue
                        let new_val = Box::into_raw(Box::new(CellValue::Empty));
                        if cas2_w(node, create_safe_idx(safe, h), val, create_safe_idx(false, h + RING_SIZE as u64), new_val) {
                            unsafe {
                                let boxs = Box::from_raw(val);
                                if let CellValue::Value(ref r_val) = *boxs {
                                    trace!("Inner dequeue: dequeue was a success");
                                    // trace!("{:?}: derefing and returning item", std::thread::current().id());
                                    return Some(std::ptr::read(r_val.assume_init_ref() as *const _));
                                }
                            }
                        } else { unsafe { drop(Box::from_raw(new_val)); } }
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
                    // Unsure if this is how they meant.
                    // let tail = self.tail.load(SeqCst);
                    // if tail > h {
                    //     for _ in 0..1000 {
                    //         std::hint::spin_loop();
                    //     }
                    //     println!("done spinning");
                    // } else {
                    //     println!("skipping");
                    // }

                    // if let CellValue::Empty = *val {
                    let new_val = Box::into_raw(Box::new(CellValue::Empty));
                    if cas2_w(node, create_safe_idx(safe, idx), val, create_safe_idx(safe, h + RING_SIZE as u64), new_val) {
                        // // Line 52
                        // println!("{:?}: cas2 success", std::thread::current().id());
                        break;
                    } else {/* println!("{:?}: cas2 failure", std::thread::current().id());  */drop(Box::from_raw(new_val)) }

                    // }
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
    fn enqueue(&self, item: *mut CellValue<T>) -> Result<(), T>{
        trace!("Starting inner enqueue now");
        loop {
            let t = self.tail.fetch_add(1, SeqCst);
            let closed = self.closed.load(SeqCst);
            if closed {
                trace!("Inner enqueue: CRQ closed.");
                unsafe {
                    if let CellValue::Value(ref val) = *Box::from_raw(item) {
                        return Err(std::ptr::read(val.assume_init_ref() as *const _));
                    }
                }
            }
            let index = t as usize % RING_SIZE;
            trace!("Inner enqueue: index: {index}");
            let node = &self.ring[t as usize % RING_SIZE];
            let val = unsafe { node.value.load(SeqCst).as_ref().unwrap() };
            let (safe, idx) = node.safe_and_idx();
            if matches!(val, CellValue::Empty) {
                trace!("Inner enqueue: val was empty");
                trace!("Inner enqueue: idx:{idx} t:{t}");
                let val_ptr = val  as *const CellValue<T> as *mut CellValue<T>;
                if idx <= t &&
                   (safe || self.head.load(SeqCst) <= t) &&
                   cas2_w(node, create_safe_idx(safe, idx), val_ptr, create_safe_idx(true, t), item) {
                    trace!("Inner enqueue: Enqueue success");
                    return Ok(());
                }
            }
            let h = self.head.load(SeqCst);
            if t >= h && t - h >= RING_SIZE as u64 {
                self.closed.store(true, SeqCst);
                unsafe {
                    if let CellValue::Value(ref val) = *Box::from_raw(item) {
                        return Err(std::ptr::read(val.assume_init_ref() as *const _));
                    }
                }
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

fn cas2_w<T>(
    node: &Cell<T>,
    safe_and_idx: u64,
    val: *mut CellValue<T>,
    new_safe_and_idx: u64,
    new_val: *mut CellValue<T>
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
            hp2: HazardPointer::new(),
        } 
    }
}

struct LCRQueueHandle<'a, T: std::fmt::Debug> {
    queue: &'a LCRQueue<T>,
    hp1: HazardPointer<'static>,
    hp2: HazardPointer<'static>, 
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

    use super::*;

    #[test]
    fn create_lcrqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
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
    fn enqueue_full_crq2() {

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
