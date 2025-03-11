use crate::ConcurrentQueue;
use std::sync::atomic::{AtomicPtr, Ordering, AtomicUsize};

#[derive(Copy)]
enum TempVal<T> {
    Val(T),
    Null(bool)
}

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
struct PQueue<T: Clone + Copy> {
    head:   AtomicUsize,
    nodes:  Vec<Node<T>>,
    tail:   AtomicUsize,
    vnull:  TempVal<T>,
}

impl<T: Clone + Copy> PQueue<T> {
    fn new(capacity: usize) -> Self {
        let mut v = vec![];
        for _ in 0..capacity {
            v.push(Node::new());
        }
        // let h = Box::into_raw(Box::new(TempVal::Val(0)));
        // let t = Box::into_raw(Box::new(TempVal::Val(1)));
        PQueue {
            // head: AtomicPtr::new(h),
            // tail: AtomicPtr::new(t),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(1),
            vnull: TempVal::Null(true),
            nodes: v,
        }
    }

    fn enqueue(&mut self, item: T) -> bool{
        loop {
            let te = self.tail.load(Ordering::Relaxed);
            let mut ate = te;
            let mut tt = &self.nodes[ate];
            let mut temp: usize = ate + 1 % self.nodes.len();
            // While we have a value and not a null
            while let TempVal::Val(val) = unsafe { *tt.val.load(Ordering::Relaxed) } {
                // check tails consistency (juicy tail)
                if te != self.tail.load(Ordering::Relaxed) { break; }
                if te != self.head.load(Ordering::Relaxed) { break; }
                tt = &self.nodes[temp];
                ate = temp;
            }
            // while (tt.val != TempVal::Null(false) && tt.val != TempVal::Null(true)) {
                
            // }
        }

        false
    }

    fn dequeue(&self) -> Option<T>{
        None
    }
}

impl<T: Clone + Copy> ConcurrentQueue<T> for PQueue<T> {
    fn new(c: usize) -> Self {
        PQueue::new(c as usize)
    }
    fn get_id(&self) -> String {
        String::from("philippas_queue")
    }
    fn register(&self) -> impl crate::Handle<T> {
        todo!();
        return crate::queues::BasicQueue::new();
    }
}