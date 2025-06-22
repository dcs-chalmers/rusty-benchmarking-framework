
use crate::traits::{ConcurrentQueue, Handle};
use faaa_queue::FAAAQueue;

pub struct FAAAQueueRSHandle<'a, T>{
    queue: & 'a FAAAQRS< T>
}

pub struct FAAAQRS<T>{
    pub faaaqueue: FAAAQueue<T>
}

impl <T> ConcurrentQueue<T> for FAAAQRS<T>{
    fn register(&self) -> impl Handle<T>{
        FAAAQueueRSHandle{
            queue: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("faaa_queue")
    }

    fn new(_size: usize) -> Self {
        FAAAQRS {
            faaaqueue: FAAAQueue::new(),
        }
    }
}

impl <T> Handle<T> for FAAAQueueRSHandle<'_, T> {
    fn push(&mut self, value: T) -> Result<(), T> {
        self.queue.faaaqueue.enqueue(value);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.faaaqueue.dequeue()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bq() {
        let q: FAAAQRS<i32> = FAAAQRS::new(100);
        q.faaaqueue.enqueue(1);
        assert_eq!(q.faaaqueue.dequeue().unwrap(), 1);
    }
    #[test]
    fn register_bq() {
        let q: FAAAQRS<i32> = FAAAQRS::new(100);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let q: FAAAQRS<i32> = FAAAQRS::new(100);
        if crate::order::benchmark_order_i32(q, 10, 5, true, 10).is_err() {
            panic!();
        }
    }
}
