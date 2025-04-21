use std::{mem::MaybeUninit, ptr::null, sync::atomic::{AtomicBool, AtomicPtr as RawAtomicPtr, AtomicU64, AtomicUsize, Ordering}};
use std::sync::atomic::Ordering::SeqCst as SeqCst;
use haphazard::{raw::Pointer, AtomicPtr as HpAtomicPtr, HazardPointer};
use log::trace;

use crate::traits::{ConcurrentQueue, Handle};

static RING_SIZE: u64 = 1024;
static MAX_THREADS: usize = 256;

thread_local! {
    static THREAD_ID: std::cell::Cell<Option<usize>> = const {std::cell::Cell::new(None)};
}


struct LPRQueue<E> {
    head: PRQ<E>,
    tail: PRQ<E>,
}

impl<E> LPRQueue<E> {
    fn enqueue(&self, item: E) {
        let mut inner_item = item;
        loop {
            let prq = &self.tail;
            match prq.enqueue(inner_item) {
                Ok(_) => return,
                Err(val) => inner_item = val,
            };
        }
    }
}

enum CellValue<E> {
    Empty,
    ThreadToken(usize),
    Value(MaybeUninit<E>),
}

struct Cell<E> {
    safe_and_epoch: AtomicU64,
    value: RawAtomicPtr<CellValue<E>>,
}

impl<E> Cell<E> {
    fn new() -> Self {
        Self {
            safe_and_epoch: AtomicU64::new(1),
            value: RawAtomicPtr::new(std::ptr::null_mut()),
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
    next_thread_id: AtomicUsize,
}

impl<E> PRQ<E> {
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
    fn enqueue(&self, item: E) -> Result<(), E>{
        let item_ptr = Box::into_raw(Box::new(CellValue::Value(MaybeUninit::new(item))));
        loop {
            let t = self.tail.fetch_add(1, Ordering::SeqCst);
            if self.closed.load(Ordering::SeqCst) { 
                if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                    return Err(unsafe{val.assume_init()});
                }
            }
            let cycle: u64 = t / RING_SIZE;
            let i: usize = (t % RING_SIZE) as usize;
            
            let (whole, safe, epoch) = self.A[i].safe_and_epoch();
            let value = self.A[i].value.load(Ordering::SeqCst);

            let is_empty = unsafe {
                if value.is_null() {
                    true
                } else {
                    matches!(*value, CellValue::Empty)
                }
            };
            let is_t = unsafe {
                if is_empty { 
                    false
                }
                else {matches!(*value, CellValue::ThreadToken(_))}
            };
            if is_empty || is_t &&
                epoch < cycle && (safe || self.head.load(Ordering::SeqCst) <= t)
            {
                let new_val = Box::into_raw(Box::new(CellValue::ThreadToken(self.get_thread_id())));
                if self.A[i]
                    .value
                    .compare_exchange(
                        value,
                        new_val,
                        Ordering::SeqCst,
                        Ordering::SeqCst 
                        ).is_err() {
                    if check_overflow(t, self.head.load(SeqCst), &self.closed) {
                        continue;
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                            return Err(unsafe{val.assume_init()});
                        }
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
                    unsafe {
                        if let CellValue::ThreadToken(token) = *value {
                            if token == self.get_thread_id() {
                                let new_val = Box::into_raw(Box::new(CellValue::Empty));
                                let _ =  self.A[i].value.compare_exchange(value, new_val, SeqCst, SeqCst);
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
                unsafe {
                    if let CellValue::ThreadToken(token) = *value {
                        if token == self.get_thread_id()
                            && self.A[i].value.compare_exchange(value, item_ptr, SeqCst, SeqCst).is_ok() 
                        {
                            return Ok(());
                        } 
                    }
                }
            }
            if !check_overflow(t, self.head.load(SeqCst), &self.closed) {
                #[allow(clippy::collapsible_if)]
                if let CellValue::Value(val) = *unsafe{Box::from_raw(item_ptr)} {
                    return Err(unsafe{val.assume_init()});
                }
            }
        }
    } 
    fn dequeue(&self) -> Option<E> {
        loop {
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
                if epoch == cycle && (!is_empty || !is_t) {
                    self.A[i].value.store(to_raw(CellValue::Empty), SeqCst);
                    let boxs = unsafe {
                        Box::from_raw(value)
                    };
                    if let CellValue::Value(val) = *boxs{
                        return Some(unsafe {std::ptr::read(val.assume_init_ref())});
                    }
                } else if epoch <= cycle && (is_empty || is_t) {
                    if is_t 
                        && self.A[i].value.compare_exchange(value, to_raw(CellValue::Empty), SeqCst, SeqCst).is_err()
                    {
                        continue;
                    }
                    let new_safe_and_epoch = (cycle << 1) | (safe as u64);
                    if self.A[i].safe_and_epoch.compare_exchange(whole, new_safe_and_epoch, SeqCst, SeqCst).is_ok() {
                        break;
                    }
                } else if epoch < cycle && (!is_empty || !is_t) {
                    let new_safe_and_epoch = (cycle << 1) | (false as u64);
                    if self.A[i].safe_and_epoch.compare_exchange(whole, new_safe_and_epoch, SeqCst, SeqCst).is_ok() {
                        break;
                    }
                } else {
                    break;
                }
            }
            if self.tail.load(SeqCst) <= h + 1 { return None }
        }
    } 
}

fn to_raw<T>(item: T) -> *mut T {
    Box::into_raw(Box::new(item))
}


fn check_overflow(t: u64, head: u64, closed: &AtomicBool) -> bool {
    if t - head >= RING_SIZE {
        closed.store(true, Ordering::SeqCst);
        return false;
    }
    true
}
