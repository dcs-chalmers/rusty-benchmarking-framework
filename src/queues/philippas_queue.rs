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
struct Node<T: Clone> {
    val: AtomicPtr<TempVal<T>>
}

impl<T: Clone + Copy> Node<T> {
    fn new() -> Self {
        let v: *mut TempVal<T> = Box::<TempVal<_>>::into_raw(Box::new(TempVal::Null(false)));
        Node {
            val: AtomicPtr::new(v)
        }
    }
}
#[derive(Debug)]
pub struct PQueue<T: Clone + Copy + Display> {
    head:   AtomicUsize,
    nodes:  Vec<Node<T>>,
    tail:   AtomicUsize,
    vnull:  AtomicPtr<TempVal<T>>,
    max_num: usize,
}

impl<T: Clone + Copy + Display + Debug + Sync + Send> PQueue<T> {
    fn new(capacity: usize) -> Self {
        let maxnum = capacity + 1;
        let mut v = vec![];
        for _ in 0..maxnum + 1 {
            v.push(Node::new());
        }
        v[0] = Node { val: AtomicPtr::new(Box::into_raw(Box::new(TempVal::Null(true))))};
        PQueue {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(1),
            vnull: AtomicPtr::new(Box::into_raw(Box::new(TempVal::Null(true)))),
            nodes: v,
            max_num: maxnum, 
        }
    }

    pub fn enqueue(&self, newnode: T) -> Result<(), T>{
        loop {
            trace!("starting enqueue");
            let te = self.tail.load(Ordering::Relaxed);
            let mut ate = te;
            let mut tt = &self.nodes[ate];
            let mut temp: usize = (ate + 1) % self.max_num;
            // While we have a value and not a null
            while let TempVal::Val(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                // check tails consistency 
                if te != self.tail.load(Ordering::Relaxed) { break; }
                if temp == self.head.load(Ordering::Relaxed) { break; }
                tt = &self.nodes[temp];
                ate = temp;
                temp = (ate + 1) % self.max_num;
            }
            // check tails consistency 
            if te != self.tail.load(Ordering::Relaxed) { continue; }
            if temp == self.head.load(Ordering::Relaxed) {
                ate = (temp + 1) % self.max_num;
                tt = &self.nodes[ate];
                if let TempVal::Val(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                    trace!("Returning false on enqueue");
                    return Err(newnode);
                }
                if ate == 0 {
                    // drop old value first?
                    // let old_val = unsafe { Box::from_raw(self.vnull.load(Ordering::Relaxed)) };
                    // drop(old_val); 
                    self.vnull.store(Box::into_raw(Box::new(unsafe { *tt.val.load(Ordering::Relaxed) })), Ordering::Relaxed);
                }
                let _ = self.head.compare_exchange(temp, ate, Ordering::Relaxed, Ordering::Relaxed);
                continue;
            }
            trace!("Values: {} {} {} {} {}", unsafe {*self.nodes[0].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[1].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[2].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[3].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[4].val.load(Ordering::Relaxed)});
            // check tails consistency 
            if te != self.tail.load(Ordering::Relaxed) { continue; }
            let new_node_ptr = Box::into_raw(Box::new(TempVal::Val(newnode)));
            if let Ok(_) = self.nodes[ate].val.compare_exchange(
                tt.val.load(Ordering::Relaxed),
                new_node_ptr,
                Ordering::Relaxed,
                Ordering::Relaxed) {
                    if (temp % 2) == 0 {
                        let _ = self.tail.compare_exchange(te, temp, Ordering::Relaxed, Ordering::Relaxed);
                    }
                trace!("temp {}, te {}, ate {}", temp, te, ate);
                return Ok(());
            }
        }
    }

    pub fn dequeue(&self) -> Option<T>{
        trace!("Start of dequeue");
        loop {
            let th = self.head.load(Ordering::Relaxed);
            let mut temp: usize = (th + 1) % self.nodes.len();
            let mut tt = &self.nodes[temp];
            
            trace!("Entering second loop");
            trace!("Values: {} {} {} {} {}", unsafe {*self.nodes[0].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[1].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[2].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[3].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[4].val.load(Ordering::Relaxed)});
            while let TempVal::Null(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                if th != self.head.load(Ordering::Relaxed) { break; }
                if temp == self.tail.load(Ordering::Relaxed) { return None;}
                temp = (temp + 1) % self.max_num;
                tt = &self.nodes[temp];
            }
            if th != self.head.load(Ordering::Relaxed) { continue; }
            if temp == self.tail.load(Ordering::Relaxed){
                let _ = self.tail.compare_exchange(temp, (temp + 1) % self.nodes.len(), Ordering::Relaxed, Ordering::Relaxed);
                continue;
            }
            let tnull: TempVal<T>;
            trace!("temp: {}, th: {}",temp, th);
            trace!("Values: {} {} {} {} {}", unsafe {*self.nodes[0].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[1].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[2].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[3].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[4].val.load(Ordering::Relaxed)});
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
            // trace!("th: {}")
            if let Ok(_) = self.nodes[temp].val.compare_exchange(
                tt.val.load(Ordering::Relaxed), 
                tnull_ptr, 
                Ordering::Relaxed, 
                Ordering::Relaxed) 
            {
                if temp == 0 {
                    self.vnull.store(Box::into_raw(Box::new(tnull)), Ordering::Relaxed); //check here for bugs as well... want to do self.vnull = tnull 
                }
                if (temp % 2) == 0 {
                    let _ = self.head.compare_exchange(th, temp, Ordering::Relaxed, Ordering::Relaxed);
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

impl<T: Clone + Copy + Display + Debug + Send + Sync> ConcurrentQueue<T> for PQueue<T> {
    fn new(c: usize) -> Self {
        PQueue::new(c as usize)
    }
    fn get_id(&self) -> String {
        String::from("philippas_queue")
    }
    fn register(&self) -> impl crate::Handle<T> {
        PQueueHandle {
            q: &self
        }
    }
}

struct PQueueHandle<'a, T: Copy + Debug + Display> {
    q: &'a PQueue<T>,
}

impl<T: Copy + Debug + Display + Send + Sync> Handle<T> for PQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        self.q.dequeue()
    }
    fn push(&mut self, item: T) -> Result<(), T>{
        self.q.enqueue(item)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use log::info;

    #[test]
    fn create_pqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: PQueue<i32> = PQueue::new(10);
        assert_eq!(q.enqueue(10), Ok(()));
        assert_eq!(q.enqueue(11), Ok(()));
        assert_eq!(q.enqueue(12), Ok(()));
        assert_eq!(q.enqueue(13), Ok(()));
        assert_eq!(q.enqueue(14), Ok(()));
        println!("{:?}", q);
        assert_eq!(q.dequeue(), Some(10));
        assert_eq!(q.dequeue(), Some(11));
        assert_eq!(q.dequeue(), Some(12));
        assert_eq!(q.dequeue(), Some(13));
        assert_eq!(q.dequeue(), Some(14));
    }
    // #[test]
    // fn multi_threaded() {
    //     let _ = env_logger::builder().is_test(true).try_init();
    //     const NUM_THREADS: usize = 2;
    //     const ITEMS_PER_THREAD: usize = 10;
    //     const QUEUE_SIZE: usize = NUM_THREADS * ITEMS_PER_THREAD;
    //     
    //     let q: PQueue<i32> = PQueue::new(QUEUE_SIZE);
    //     let barrier = std::sync::Barrier::new(NUM_THREADS * 2);
    //     
    //     
    //     let _ = std::thread::scope(|s| -> Result<(), std::io::Error> {
    //         let q = &q;
    //         let barrier = &barrier;
    //         
    //         // s.spawn(move || {
    //         //     std::thread::sleep(Duration::from_secs(10));
    //         //     assert!(false);
    //         // });
    //
    //         // Create producer threads
    //         for thread_id in 0..NUM_THREADS {
    //             s.spawn(move || {
    //                 // Wait for all threads to be ready
    //                 barrier.wait();
    //                 
    //                 let start = thread_id * ITEMS_PER_THREAD;
    //                 let end = start + ITEMS_PER_THREAD;
    //                 
    //                 for i in start..end {
    //                     let val = i as i32;
    //                     let _  = q.enqueue(val);
    //                 }
    //                 
    //                 info!("Producer {} finished", thread_id);
    //             });
    //         }
    //         
    //         // Create consumer threads
    //         for thread_id in 0..NUM_THREADS {
    //             s.spawn(move || {
    //                 // Wait for all threads to be ready
    //                 barrier.wait();
    //                 
    //                 let items_to_consume = ITEMS_PER_THREAD;
    //                 let mut consumed = 0;
    //                 
    //                 while consumed < items_to_consume {
    //                     if q.dequeue().is_some() {
    //                         consumed += 1;
    //                     }
    //                 }
    //                 
    //                 info!("Consumer {} finished, consumed {} items", thread_id, consumed);
    //             });
    //         }
    //         
    //         Ok(())
    //     });
    //     
    //     // Ensure queue is empty after all operations
    //     assert_eq!(q.dequeue(), None, "Queue should be empty after test");
    // }
    // #[test]
    // fn register_pqueue() {
    //     let q: MSQueue<i32> = MSQueue::new(1000);
    //     let mut handle = q.register();
    //     handle.push(1);
    //     assert_eq!(handle.pop().unwrap(), 1);

    // }
}
