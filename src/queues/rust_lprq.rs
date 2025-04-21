use std::sync::atomic::{AtomicBool, AtomicPtr as RawAtomicPtr, AtomicU64, AtomicUsize, Ordering};

use haphazard::{AtomicPtr as HpAtomicPtr, HazardPointer};
use log::trace;

use crate::traits::{ConcurrentQueue, Handle};

static RING_SIZE: usize = 1024;
static MAX_THREADS: usize = 256;

struct Cell<E> {
    safe_and_epoch: AtomicU64,
    value: RawAtomicPtr<Option<E>>,
}

impl<E> Cell<E> {
    fn new() -> Self {
        Self {
            safe_and_epoch: AtomicU64::new(1),
            value: RawAtomicPtr::new(Box::into_raw(Box::new(None))),
        }
    }
    
    // Getters and setters for the packed fields
    fn is_safe(&self) -> bool {
        let safe_and_epoch = self.safe_and_epoch.load(Ordering::SeqCst);
        (safe_and_epoch & 1) == 1
    }
    
    fn get_epoch(&self) -> u64 {
        self.safe_and_epoch.load(Ordering::SeqCst) >> 1
    }
    
    fn set_safe(&mut self, safe: bool) {
        let safe_and_epoch = self.safe_and_epoch.load(Ordering::SeqCst);
        if safe {
            self.safe_and_epoch.store(safe_and_epoch | 1, Ordering::SeqCst);
        } else {
            self.safe_and_epoch.store(safe_and_epoch & !1, Ordering::SeqCst);
        }
    }
    
    fn set_epoch(&mut self, epoch: u64) {
        // Clear the epoch bits but preserve the safe bit
        self.safe_and_epoch.store(
            (epoch << 1) | (self.safe_and_epoch.load(Ordering::SeqCst) & 1),
            Ordering::SeqCst);
    }
}

#[allow(non_snake_case)]
#[allow(clippy::upper_case_acronyms)]
struct PRQ<E> {
    next: RawAtomicPtr<PRQ<E>>,
    closed: AtomicBool,
    A: Vec<Cell<E>>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<E> PRQ<E> {
    
    fn enqueue(&self, item: E) -> bool {
        loop {
            let t = self.tail.fetch_add(1, Ordering::SeqCst);
            if self.closed.load(Ordering::SeqCst) { return false }
            let cycle = t / RING_SIZE;
            let i = t % RING_SIZE;
            todo!()
        }
    } 
}
