use concurrent_queue::PushError;
use log::warn;

use benchmark_core::traits::{ConcurrentQueue, HandleQueue};

pub struct UnboundedCQueue<T> {
    q: concurrent_queue::ConcurrentQueue<T>,
}

pub struct UnboundedCQueueHandle<'a, T> {
    queue: &'a UnboundedCQueue<T>
}

impl<T> ConcurrentQueue<T> for UnboundedCQueue<T> {
    fn register(&self) -> impl HandleQueue<T> {
        UnboundedCQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("unbounded_concurrent_queue")
    }
    fn new(_size: usize) -> Self {
        UnboundedCQueue {
            q: concurrent_queue::ConcurrentQueue::unbounded()
        }
    }
}

impl<T> HandleQueue<T> for UnboundedCQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        if let Err(err) = self.queue.q.push(item) {
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
        self.queue.q.pop().ok()
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
        let q: UnboundedCQueue<i32> = UnboundedCQueue::new(0);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    #[ignore]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: UnboundedCQueue<i32> = UnboundedCQueue::new(0);
        if benchmark_core::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}

