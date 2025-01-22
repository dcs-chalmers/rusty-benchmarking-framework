use crate::queues::lf_queue::LFQueue;

pub mod queues;

pub fn start_benchmark() {
    let test_q: LFQueue<i32> =  LFQueue {
        lfq: lockfree::queue::Queue::new(),
    };
    test_q.lfq.push(50);
    println!("{}", test_q.lfq.pop().unwrap())
}

pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
}

pub trait Handle<T> {
    fn push(&self, item: T);
    fn pop(&self) -> Option<T>;
}


