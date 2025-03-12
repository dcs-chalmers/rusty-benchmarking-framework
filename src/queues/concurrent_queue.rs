use concurrent_queue::PushError;
use log::warn;

use crate::{ConcurrentQueue, Handle};

pub struct CQueueHandle<'a, T> {
    queue: &'a CQueue<T>
}

pub struct CQueue<T> {
    pub cq: concurrent_queue::ConcurrentQueue<T>
}

impl<T> ConcurrentQueue<T> for CQueue<T> {
    fn register(&self) -> impl Handle<T> {
        CQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        return String::from("ConcurrentQueue")
    }
    fn new(size: usize) -> Self {
        CQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(size),
        }
    }
}

impl<T> Handle<T> for CQueueHandle<'_, T> {
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
        match self.queue.cq.pop() {
            Ok(val) => Some(val),
            Err(_) => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_cqueue() {
        let q: CQueue<i32> = CQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(100)
        };
        let _ = q.cq.push(1);
        assert_eq!(q.cq.pop().unwrap(), 1);
    }
    #[test]
    fn test_handle() {
        let q: CQueue<i32> = CQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(100)
        };
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    #[should_panic]
    fn test_too_small() {
        let q: CQueue<i32> = CQueue {
            cq: concurrent_queue::ConcurrentQueue::bounded(1)
        };
        let _ = q.cq.push(1);
        q.cq.push(1).unwrap();
    }
}
