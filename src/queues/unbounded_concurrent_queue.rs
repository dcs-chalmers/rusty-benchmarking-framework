use concurrent_queue::PushError;
use log::warn;

use crate::traits::{ConcurrentQueue, Handle};

pub struct UnboundedCQueueHandle<'a, T> {
    queue: &'a concurrent_queue::ConcurrentQueue<T> 
}

impl<T> ConcurrentQueue<T> for concurrent_queue::ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T> {
        UnboundedCQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("unbounded_concurrent_queue")
    }
    fn new(_size: usize) -> Self {
            concurrent_queue::ConcurrentQueue::unbounded()
    }
}

impl<T> Handle<T> for UnboundedCQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        if let Err(err) = self.queue.push(item) {
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
        self.queue.pop().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_cqueue() {
        let q: concurrent_queue::ConcurrentQueue<i32> = concurrent_queue::ConcurrentQueue::unbounded();
        let _ = q.push(1);
        assert_eq!(q.pop().unwrap(), 1);
    }
    #[test]
    fn test_handle() {
        let q: concurrent_queue::ConcurrentQueue<i32> = concurrent_queue::ConcurrentQueue::unbounded();
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: concurrent_queue::ConcurrentQueue<i32> = concurrent_queue::ConcurrentQueue::unbounded();
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}

