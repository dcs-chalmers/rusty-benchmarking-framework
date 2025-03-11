use log::{debug, error, trace};

use crate::ConcurrentQueue;
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
struct PQueue<T: Clone + Copy + Display> {
    head:   AtomicUsize,
    nodes:  Vec<Node<T>>,
    tail:   AtomicUsize,
    vnull:  TempVal<T>,
    MAXNUM: usize,
}

impl<T: Clone + Copy + Display + Debug> PQueue<T> {
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
            vnull: TempVal::Null(true),
            nodes: v,
            MAXNUM: maxnum, 
        }
    }

    fn enqueue(&mut self, newnode: T) -> bool{
        loop {
            trace!("starting enqueue");
            let te = self.tail.load(Ordering::Relaxed);
            let mut ate = te;
            let mut tt = &self.nodes[ate];
            let mut temp: usize = (ate + 1) % self.MAXNUM;
            // While we have a value and not a null
            while let TempVal::Val(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                // check tails consistency 
                if te != self.tail.load(Ordering::Relaxed) { break; }
                if temp == self.head.load(Ordering::Relaxed) { break; }
                tt = &self.nodes[temp];
                ate = temp;
                temp = (ate + 1) % self.MAXNUM;
            }
            // check tails consistency 
            if te != self.tail.load(Ordering::Relaxed) { continue; }
            if temp == self.head.load(Ordering::Relaxed) {
                ate = (temp + 1) % self.MAXNUM;
                tt = &self.nodes[ate];
                if let TempVal::Val(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                    debug!("Returning false on enqueue");
                    trace!("{:?}", self);
                    return false;
                }
                if ate == 0 {
                    self.vnull = unsafe { *tt.val.load(Ordering::Relaxed) };
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
                return true;
            }
        }
    }

    fn dequeue(&mut self) -> Option<T>{
        println!("Start of dequeue");
        loop {
            let th = self.head.load(Ordering::Relaxed);
            let mut temp: usize = (th + 1) % self.nodes.len();
            let mut tt = &self.nodes[temp];
            
            println!("Entering second loop");
            trace!("Values: {} {} {} {} {}", unsafe {*self.nodes[0].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[1].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[2].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[3].val.load(Ordering::Relaxed)}, unsafe {*self.nodes[4].val.load(Ordering::Relaxed)});
            while let TempVal::Null(_) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                if th != self.head.load(Ordering::Relaxed) { println!("breaking now"); break; }
                println!("We are here now lol");
                if temp == self.tail.load(Ordering::Relaxed) { return None;}
                temp = (temp + 1) % self.MAXNUM;
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
                    tnull = self.vnull;
                }
            } else {
                tnull = match self.vnull {
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
                    self.vnull = tnull;
                }
                if (temp % 2) == 0 {
                    let _ = self.head.compare_exchange(th, temp, Ordering::Relaxed, Ordering::Relaxed);
                }
                match real_tt {
                    TempVal::Null(_) => {
                        error!("Return value was a null");
                        println!("Returning none");
                        return None;
                    },
                    TempVal::Val(v) => return Some(v),
                }
            }
        }
    }
}

// impl<T: Clone + Copy> ConcurrentQueue<T> for PQueue<T> {
//     fn new(c: usize) -> Self {
//         PQueue::new(c as usize)
//     }
//     fn get_id(&self) -> String {
//         String::from("philippas_queue")
//     }
//     fn register(&self) -> impl crate::Handle<T> {
//         todo!();
//         return crate::queues::BasicQueue::new();
//     }
// }



#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, warn};

    #[test]
    fn create_pqueue() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut q: PQueue<i32> = PQueue::new(10);
        assert!(q.enqueue(10));
        assert!(q.enqueue(11));
        assert!(q.enqueue(12));
        assert!(q.enqueue(13));
        assert!(q.enqueue(14));
        println!("{:?}", q);
        assert_eq!(q.dequeue(), Some(10));
        assert_eq!(q.dequeue(), Some(11));
        assert_eq!(q.dequeue(), Some(12));
        assert_eq!(q.dequeue(), Some(13));
        assert_eq!(q.dequeue(), Some(14));
    }
    // #[test]
    // fn register_pqueue() {
    //     let q: MSQueue<i32> = MSQueue::new(1000);
    //     let mut handle = q.register();
    //     handle.push(1);
    //     assert_eq!(handle.pop().unwrap(), 1);

    // }
}