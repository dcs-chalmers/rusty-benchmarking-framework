#[allow(unused_imports)]
use log::{debug, trace};

use benchmark_core::traits::{ConcurrentQueue, Handle};
use std::{fmt::{Debug, Display}, sync::atomic::{AtomicUsize, Ordering}};
use haphazard::HazardPointer;

#[derive(Copy, Clone, Debug)]
enum TempVal<T> {
    Val(T),
    Null(bool)
}
impl<T: Display> Display for TempVal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TempVal::Null(a) => write!(f, "{}", a),
            TempVal::Val(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug)]
struct Node<T> {
    val: haphazard::AtomicPtr<TempVal<T>>
}

impl<T> Node<T> {
    fn new() -> Self {
        let v = Box::new(TempVal::Null(false));
        Node {
            val: haphazard::AtomicPtr::from(v)
        }
    }
}
#[derive(Debug)]
pub struct TZQueue<T> {
    head:   AtomicUsize,
    nodes:  Vec<Node<T>>,
    tail:   AtomicUsize,
    vnull:  haphazard::AtomicPtr<TempVal<T>>,
    max_num: usize,
}

impl<T:Sync + Send + Copy + Display> TZQueue<T> {
    fn new(capacity: usize) -> Self {
        let max_num = capacity + 1;
        let mut v = Vec::with_capacity(max_num + 1);
        v.push(Node {
            val: haphazard::AtomicPtr::from(Box::new(TempVal::Null(true)))
        });
        for _ in 0..max_num + 1 {
            v.push(Node::new());
        }
        TZQueue {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(1),
            vnull: haphazard::AtomicPtr::from(Box::new(TempVal::Null(true))),
            nodes: v,
            max_num,
        }
    }

    pub fn enqueue(&self, newnode: T, hp1: &mut HazardPointer) -> Result<(), T>{
        loop {
            trace!("starting enqueue");
            // self.print_queue();
            // Read the tail
            let te = self.tail.load(Ordering::SeqCst);
            let mut ate = te;
            // Get reference to node
            let mut tt = self.nodes[ate].val.safe_load(hp1).unwrap();
            // Next after tail
            let mut temp: usize = (ate + 1) % (self.max_num + 1);
            // While we have a value and not a null.
            // Try to find the actual tail.
            trace!("{te} {ate} {temp}");
            while let TempVal::Val(_) = *tt {
                // check tails consistency
                trace!("trying to find actual tail");
                if te != self.tail.load(Ordering::SeqCst) { break; }
                if temp == self.head.load(Ordering::SeqCst) { break; }
                hp1.reset_protection();
                tt = self.nodes[temp].val.safe_load(hp1).unwrap();
                ate = temp;
                temp = (ate + 1) % (self.max_num + 1);
            }
            // check tails consistency
            if te != self.tail.load(Ordering::SeqCst) { continue; }
            // Check wether queue is full
            if temp == self.head.load(Ordering::SeqCst) {
                ate = (temp + 1) % (self.max_num + 1);
                hp1.reset_protection();
                tt = self.nodes[ate].val.safe_load(hp1).unwrap();
                // If the node after the head is occupied, then queue is full
                if let TempVal::Val(_) = *tt {
                    trace!("Queue was full: ate {} temp {} maxnum {}", ate , temp, self.max_num);
                    return Err(newnode);
                }
                if ate == 0 {
                    let old_val = self.vnull.swap(Box::new(*tt))
                        .expect("vnull is never null");
                    // Safety: Safe since we have swapped the value atomically.
                    unsafe { old_val.retire(); }
                }
                let _ = self.head.compare_exchange_weak(temp, ate, Ordering::SeqCst, Ordering::SeqCst);
                continue;
            }
            // check tails consistency
            if te != self.tail.load(Ordering::SeqCst) { continue; }
            let new_node_ptr = Box::into_raw(Box::new(TempVal::Val(newnode)));
            if let Ok(old_val) = unsafe {
                self.nodes[ate].val.compare_exchange_ptr(tt as *const TempVal<T> as *mut TempVal<T>, new_node_ptr)
            }{
                    if (temp % 2) == 0 {
                        let _ = self.tail.compare_exchange_weak(te, temp, Ordering::SeqCst, Ordering::SeqCst);
                    }
                unsafe { old_val.expect("CAS passed").retire(); } //BUG WAS HERE
                return Ok(());
            }
        }
    }
    #[allow(dead_code)]
    fn print_queue(&self) {
        for node in &self.nodes {
            trace!("{}", unsafe { *node.val.load_ptr() } );
        }
    }
    pub fn dequeue(&self, hp1: &mut HazardPointer) -> Option<T>{
        loop {
            trace!("starting dequeue");
            // self.print_queue();
            let th = self.head.load(Ordering::SeqCst);
            // temp is the index we want to dequeue
            let mut temp: usize = (th + 1) % (self.max_num + 1);
            let mut tt = self.nodes[temp].val.safe_load(hp1).unwrap();
            // Find the actual head
            while let TempVal::Null(_) = *tt {
                if th != self.head.load(Ordering::SeqCst) { break; }
                if temp == self.tail.load(Ordering::SeqCst) { return None;}
                temp = (temp + 1) % (self.max_num + 1);
                hp1.reset_protection();
                tt = self.nodes[temp].val.safe_load(hp1).unwrap();
            }
            if th != self.head.load(Ordering::SeqCst) { continue; }
            if temp == self.tail.load(Ordering::SeqCst){
                let _ = self.tail.compare_exchange_weak(temp, (temp + 1) % (self.max_num + 1), Ordering::SeqCst, Ordering::SeqCst);
                continue;
            }
            let tnull: TempVal<T>;
            if temp != 0 {
                if temp < th {
                    trace!("Setting tnull to node 0 val");
                    tnull = unsafe { *self.nodes[0].val.load_ptr() };
                } else {
                    trace!("Setting tnull to value of vnull");
                    // tnull = *self.vnull.safe_load(hp2).unwrap();
                    tnull = unsafe { *self.vnull.load_ptr() };
                }
            } else {
                tnull = match unsafe { *self.vnull.load_ptr() }  {
                    TempVal::Null(b) => TempVal::Null(!b),
                    TempVal::Val(v) => TempVal::Val(v),
                }
            }
            if th != self.head.load(Ordering::SeqCst){ continue; }
            let tnull_ptr = Box::into_raw(Box::new(tnull));
            if let Ok(old_val) = unsafe {self.nodes[temp].val.compare_exchange_ptr(tt as *const TempVal<T> as *mut TempVal<T>, tnull_ptr) }
            {
                if temp == 0 {
                    let old_val = self.vnull.swap(Box::new(tnull)).expect("vnull is never null");
                    trace!("dropping {}", unsafe { *old_val.as_ptr()});
                    unsafe { old_val.retire(); }
                }
                if (temp % 2) == 0 {
                    let _ = self.head.compare_exchange_weak(th, temp, Ordering::SeqCst, Ordering::SeqCst);
                }
                // Safety: old_val is never null.
                let r_val = unsafe { *old_val.unwrap().as_ptr() };
                // Safety: CAS succeeded so no new references to old_val.
                // This is the only thread that will retire.
                unsafe { old_val.unwrap().retire(); }
                match r_val {
                    TempVal::Null(_) => {
                        trace!("Return value was a null from dequeue");
                        return None;
                    },
                    TempVal::Val(v) => return Some(v),
                }
            }
        }
    }
}

impl<T: Copy + Send + Sync + Display> ConcurrentQueue<T> for TZQueue<T> {
    fn new(c: usize) -> Self {
        TZQueue::new(c)
    }
    fn get_id(&self) -> String {
        String::from("tz_queue_hp")
    }
    fn register(&self) -> impl benchmark_core::traits::Handle<T> {
        TZQueueHandle {
            q:      self,
            hp1:    HazardPointer::new(),
        }
    }
}

struct TZQueueHandle<'a, T: Copy> {
    q: &'a TZQueue<T>,
    hp1: HazardPointer<'static>,
}

impl<T: Copy + Send + Sync + Display> Handle<T> for TZQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        self.q.dequeue(&mut self.hp1)
    }
    fn push(&mut self, item: T) -> Result<(), T>{
        self.q.enqueue(item, &mut self.hp1)
    }
}

impl<T> Drop for TZQueue<T> {
    fn drop(&mut self) {
        trace!("Starting drop function for TZQueue");
        let reclaim_vnull = unsafe { Box::from_raw(self.vnull.load_ptr()) };
        trace!("Dropping vnull now");
        drop(reclaim_vnull);
        trace!("Starting dropping of nodes");
        for node in &self.nodes {
            let reclaimed_node_val = unsafe { Box::from_raw(node.val.load_ptr()) };
            trace!("dropping node");
            drop(reclaimed_node_val); //time to eat, ah you mean lunch xd, trodde du drog en rolig kommentar om drop xd XD
        }
        trace!("Done with drop function");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use log::info;

    #[test]
    fn create_tzqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut hp1 = haphazard::HazardPointer::new();
        let q: TZQueue<i32> = TZQueue::new(5);
        assert_eq!(q.enqueue(10, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(11, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(12, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(13, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(14, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(15, &mut hp1), Err(15));
        println!("{:?}", q);
        assert_eq!(q.dequeue(&mut hp1), Some(10));
        assert_eq!(q.dequeue(&mut hp1), Some(11));
        assert_eq!(q.dequeue(&mut hp1), Some(12));
        assert_eq!(q.dequeue(&mut hp1), Some(13));
        assert_eq!(q.dequeue(&mut hp1), Some(14));
        assert_eq!(q.dequeue(&mut hp1), None);
        assert_eq!(q.enqueue(16, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(17, &mut hp1), Ok(()));
        assert_eq!(q.dequeue(&mut hp1), Some(16));
        assert_eq!(q.enqueue(18, &mut hp1), Ok(()));
        assert_eq!(q.dequeue(&mut hp1), Some(17));

    }
    #[test]
    #[ignore]
    fn multi_threaded() {
        let _ = env_logger::builder().is_test(true).try_init();
        const NUM_THREADS: usize = 2;
        const ITEMS_PER_THREAD: usize = 10;
        const QUEUE_SIZE: usize = NUM_THREADS * ITEMS_PER_THREAD;

        let q: TZQueue<i32> = TZQueue::new(QUEUE_SIZE);
        let barrier = std::sync::Barrier::new(NUM_THREADS * 2);


        let _ = std::thread::scope(|s| -> Result<(), std::io::Error> {
            let q = &q;
            let barrier = &barrier;

            // Create producer threads
            for thread_id in 0..NUM_THREADS {
                s.spawn(move || {
                    // Wait for all threads to be ready
                    barrier.wait();
                    let mut hp = HazardPointer::new();
                    let start = thread_id * ITEMS_PER_THREAD;
                    let end = start + ITEMS_PER_THREAD;

                    for i in start..end {
                        let val = i as i32;
                        let _  = q.enqueue(val, &mut hp);
                    }

                    info!("Producer {} finished", thread_id);
                });
            }

            // Create consumer threads
            for thread_id in 0..NUM_THREADS {
                s.spawn(move || {
                    // Wait for all threads to be ready
                    barrier.wait();
                    let mut hp1 = HazardPointer::new();

                    let items_to_consume = ITEMS_PER_THREAD;
                    let mut consumed = 0;

                    while consumed < items_to_consume {
                        if q.dequeue(&mut hp1).is_some() {
                            consumed += 1;
                        }
                    }

                    info!("Consumer {} finished, consumed {} items", thread_id, consumed);
                });
            }

            Ok(())
        });

        let mut hp1 = HazardPointer::new();
        // Ensure queue is empty after all operations
        assert_eq!(q.dequeue(&mut hp1), None, "Queue should be empty after test");
    }
    #[test]
    fn register_tzqueue() {
        let q: TZQueue<i32> = TZQueue::new(1);
        let mut handle = q.register();
        assert_eq!(handle.push(1), Ok(()));
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn basic_drop_test() {
        let q: TZQueue<i32> = TZQueue::new(5);
        let mut hp1 = haphazard::HazardPointer::new();
        assert_eq!(q.enqueue(1, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(2, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(3, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(4, &mut hp1), Ok(()));
        assert_eq!(q.enqueue(5, &mut hp1), Ok(()));
        drop(q);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: TZQueue<i32> = TZQueue::new(1000000);
        if benchmark_core::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
