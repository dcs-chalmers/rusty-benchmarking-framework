#[allow(unused_imports)]
use log::{debug, trace};

use crate::{ConcurrentQueue, Handle};
use std::{fmt::{Debug, Display}, sync::atomic::{AtomicPtr, AtomicUsize, Ordering}};

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
    val: AtomicPtr<TempVal<T>>
}

impl<T> Node<T> {
    fn new() -> Self {
        let v: *mut TempVal<T> = Box::<TempVal<_>>::into_raw(Box::new(TempVal::Null(false)));
        Node {
            val: AtomicPtr::new(v)
        }
    }
}
#[derive(Debug)]
pub struct TZQueue<T> {
    head:   AtomicUsize,
    nodes:  Vec<Node<T>>,
    tail:   AtomicUsize,
    vnull:  AtomicPtr<TempVal<T>>,
    max_num: usize,
}

impl<T:Sync + Send + Copy + Display> TZQueue<T> {
    fn new(capacity: usize) -> Self {
        let max_num = capacity + 1;
        let mut v = vec![];
        for _ in 0..max_num + 1 {
            v.push(Node::new());
        }
        v[0] = Node { val: AtomicPtr::new(Box::into_raw(Box::new(TempVal::Null(true))))};
        TZQueue {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(1),
            vnull: AtomicPtr::new(Box::into_raw(Box::new(TempVal::Null(true)))),
            nodes: v,
            max_num, 
        }
    }

    pub fn enqueue(&self, newnode: T) -> Result<(), T>{
        loop {
            trace!("starting enqueue");
            self.print_queue();
            // Read the tail
            let te = self.tail.load(Ordering::Relaxed);
            let mut ate = te;
            // Get reference to node
            let mut tt = &self.nodes[ate];
            // Next after tail
            let mut temp: usize = (ate + 1) % (self.max_num + 1);
            // While we have a value and not a null.
            // Try to find the actual tail.
            trace!("{te} {ate} {temp}");
            while let TempVal::Val(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                // check tails consistency 
                trace!("trying to find actual tail");
                if te != self.tail.load(Ordering::Relaxed) { break; }
                if temp == self.head.load(Ordering::Relaxed) { break; }
                tt = &self.nodes[temp];
                ate = temp;
                temp = (ate + 1) % (self.max_num + 1);
            }
            // check tails consistency 
            if te != self.tail.load(Ordering::Relaxed) { continue; }
            // Check wether queue is full
            if temp == self.head.load(Ordering::Relaxed) {
                ate = (temp + 1) % (self.max_num + 1);
                tt = &self.nodes[ate];
                // If the node after the head is occupied, then queue is full
                if let TempVal::Val(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                    trace!("Queue was full: ate {} temp {} maxnum {}", ate , temp, self.max_num);
                    return Err(newnode);
                }
                if ate == 0 {
                    // drop old value first?
                    // let old_val = unsafe { Box::from_raw(self.vnull.load(Ordering::Relaxed)) };
                    // drop(old_val); 
                    self.vnull.store(Box::into_raw(Box::new(unsafe { *tt.val.load(Ordering::Relaxed) })), Ordering::Relaxed);
                }
                let _ = self.head.compare_exchange_weak(temp, ate, Ordering::Relaxed, Ordering::Relaxed);
                continue;
            }
            // check tails consistency 
            if te != self.tail.load(Ordering::Relaxed) { continue; }
            let new_node_ptr = Box::into_raw(Box::new(TempVal::Val(newnode)));
            if self.nodes[ate].val.compare_exchange_weak(
                tt.val.load(Ordering::Relaxed),
                new_node_ptr,
                Ordering::Relaxed,
                Ordering::Relaxed).is_ok() {
                    if (temp % 2) == 0 {
                        let _ = self.tail.compare_exchange_weak(te, temp, Ordering::Relaxed, Ordering::Relaxed);
                    }
                return Ok(());
            }
        }
    }
    fn print_queue(&self) {
        for node in &self.nodes {
            trace!("{}", unsafe { *node.val.load(Ordering::Relaxed) } );
        }
    }
    pub fn dequeue(&self) -> Option<T>{
        loop {
            trace!("starting dequeue");
            self.print_queue();
            let th = self.head.load(Ordering::Relaxed);
            // temp is the index we want to dequeue
            let mut temp: usize = (th + 1) % (self.max_num + 1);
            let mut tt = &self.nodes[temp];
            // Find the actual head 
            while let TempVal::Null(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                if th != self.head.load(Ordering::Relaxed) { break; }
                if temp == self.tail.load(Ordering::Relaxed) { return None;}
                temp = (temp + 1) % (self.max_num + 1);
                tt = &self.nodes[temp];
            }
            if th != self.head.load(Ordering::Relaxed) { continue; }
            if temp == self.tail.load(Ordering::Relaxed){
                let _ = self.tail.compare_exchange_weak(temp, (temp + 1) % (self.max_num + 1), Ordering::Relaxed, Ordering::Relaxed);
                continue;
            }
            let tnull: TempVal<T>;
            if temp != 0 {
                if temp < th {
                    trace!("Setting tnull to node 0 val");
                    tnull = unsafe { *self.nodes[0].val.load(Ordering::Relaxed) };
                } else {
                    trace!("Setting tnull to value of vnull");
                    tnull = unsafe { *self.vnull.load(Ordering::Relaxed) }; // Check 142 for bugs, was tnull = self.vnull
                }
            } else {
                tnull = match unsafe { *self.vnull.load(Ordering::Relaxed) } { // check for bugs 145, was match self.vnull
                    TempVal::Null(b) => TempVal::Null(!b),
                    TempVal::Val(v) => TempVal::Val(v),
                }
            }
            if th != self.head.load(Ordering::Relaxed){ continue; }
            let tnull_ptr = Box::into_raw(Box::new(tnull));
            let real_tt = unsafe { *tt.val.load(Ordering::Relaxed) };
            if self.nodes[temp].val.compare_exchange_weak(
                tt.val.load(Ordering::Relaxed), 
                tnull_ptr, 
                Ordering::Relaxed, 
                Ordering::Relaxed).is_ok() 
            {
                if temp == 0 {
                    self.vnull.store(Box::into_raw(Box::new(tnull)), Ordering::Relaxed); //check here for bugs as well... want to do self.vnull = tnull 
                }
                if (temp % 2) == 0 {
                    let _ = self.head.compare_exchange_weak(th, temp, Ordering::Relaxed, Ordering::Relaxed);
                }
                match real_tt {
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
        String::from("tz_queue")
    }
    fn register(&self) -> impl crate::Handle<T> {
        TZQueueHandle {
            q: self
        }
    }
}

struct TZQueueHandle<'a, T: Copy> {
    q: &'a TZQueue<T>,
}

impl<T: Copy + Send + Sync + Display> Handle<T> for TZQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        self.q.dequeue()
    }
    fn push(&mut self, item: T) -> Result<(), T>{
        self.q.enqueue(item)
    }
}

impl<T> Drop for TZQueue<T> {
    fn drop(&mut self) {
        trace!("Starting drop function for TZQueue");
        let reclaim_vnull = unsafe { Box::from_raw(self.vnull.load(Ordering::Relaxed)) };
        trace!("Dropping vnull now");
        drop(reclaim_vnull);
        trace!("Starting dropping of nodes");
        for node in &self.nodes {
            let reclaimed_node_val = unsafe { Box::from_raw(node.val.load(Ordering::Relaxed)) };
            drop(reclaimed_node_val);
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
    fn create_pqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: TZQueue<i32> = TZQueue::new(5);
        assert_eq!(q.enqueue(10), Ok(()));
        assert_eq!(q.enqueue(11), Ok(()));
        assert_eq!(q.enqueue(12), Ok(()));
        assert_eq!(q.enqueue(13), Ok(()));
        assert_eq!(q.enqueue(14), Ok(()));
        assert_eq!(q.enqueue(15), Err(15));
        println!("{:?}", q);
        assert_eq!(q.dequeue(), Some(10));
        assert_eq!(q.dequeue(), Some(11));
        assert_eq!(q.dequeue(), Some(12));
        assert_eq!(q.dequeue(), Some(13));
        assert_eq!(q.dequeue(), Some(14));
        assert_eq!(q.dequeue(), None);
        assert_eq!(q.enqueue(16), Ok(()));
        assert_eq!(q.enqueue(17), Ok(()));
        assert_eq!(q.dequeue(), Some(16));
        assert_eq!(q.enqueue(18), Ok(()));
        assert_eq!(q.dequeue(), Some(17));

    }
    #[test]
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
                    let start = thread_id * ITEMS_PER_THREAD;
                    let end = start + ITEMS_PER_THREAD;
                    
                    for i in start..end {
                        let val = i as i32;
                        let _  = q.enqueue(val);
                    }
                    
                    info!("Producer {} finished", thread_id);
                });
            }
            
            // Create consumer threads
            for thread_id in 0..NUM_THREADS {
                s.spawn(move || {
                    // Wait for all threads to be ready
                    barrier.wait();
                    
                    let items_to_consume = ITEMS_PER_THREAD;
                    let mut consumed = 0;
                    
                    while consumed < items_to_consume {
                        if q.dequeue().is_some() {
                            consumed += 1;
                        }
                    }
                    
                    info!("Consumer {} finished, consumed {} items", thread_id, consumed);
                });
            }
            
            Ok(())
        });
        
        // Ensure queue is empty after all operations
        assert_eq!(q.dequeue(), None, "Queue should be empty after test");
    }
    #[test]
    fn register_pqueue() {
        let q: TZQueue<i32> = TZQueue::new(1);
        let mut handle = q.register();
        assert_eq!(handle.push(1), Ok(()));
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn basic_drop_test() {
        let q: TZQueue<i32> = TZQueue::new(5);
        assert_eq!(q.enqueue(1), Ok(()));
        assert_eq!(q.enqueue(2), Ok(()));
        assert_eq!(q.enqueue(3), Ok(()));
        assert_eq!(q.enqueue(4), Ok(()));
        assert_eq!(q.enqueue(5), Ok(()));
        drop(q);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: TZQueue<i32> = TZQueue::new(10);
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
