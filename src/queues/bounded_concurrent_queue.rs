use concurrent_queue::PushError;
use log::warn;

use crate::traits::{ConcurrentQueue, Handle};

pub struct BoundedCQueueHandle<'a, T> {
    queue: &'a BoundedCQueue<T>
}

pub struct BoundedCQueue<T> {
    pub cq: concurrent_queue::ConcurrentQueue<T>
}

impl<T> ConcurrentQueue<T> for BoundedCQueue<T> {
    fn register(&self) -> impl Handle<T> {
        BoundedCQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("bounded_concurrent_queue")
    }
    fn new(size: usize) -> Self {
        BoundedCQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(size),
        }
    }
}

impl<T> Handle<T> for BoundedCQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        if let Err(err) = self.queue.cq.push(item) {
            let i = match err {
                PushError::Full(v) => {
                    warn!("Concurrentqueue was full.");
                    v
                },
                PushError::Closed(v) => {
                    warn!("Concurrentqueue was closed.");
                    v
                },
            };
            return Err(i);
        }
        Ok(())
    }
    
    fn pop(&mut self) -> Option<T> {
        self.queue.cq.pop().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_cqueue() {
        let q: BoundedCQueue<i32> = BoundedCQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(100)
        };
        let _ = q.cq.push(1);
        assert_eq!(q.cq.pop().unwrap(), 1);
    }
    #[test]
    fn test_handle() {
        let q: BoundedCQueue<i32> = BoundedCQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(100)
        };
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    #[should_panic]
    fn test_too_small() {
        let q: BoundedCQueue<i32> = BoundedCQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(1)
        };
        let _ = q.cq.push(1);
        q.cq.push(1).unwrap();
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: BoundedCQueue<i32> = BoundedCQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(1)
        };
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
