use crate::traits::{ConcurrentQueue, Handle};
use lprq_rs::LPRQueue;

pub struct LPRQueueRSHandle<'a, T>{
    queue: & 'a LPRQRS< T>
}

pub struct LPRQRS<T>{
    pub lprqueue: LPRQueue<T>
}

impl <T> ConcurrentQueue<T> for LPRQRS<T>{
    fn register(&self) -> impl Handle<T>{
        LPRQueueRSHandle{
            queue: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("lprq-rs")
    }

    fn new(_size: usize) -> Self {
        LPRQRS {
            lprqueue: LPRQueue::new(),
        }
    }
}

impl <T> Handle<T> for LPRQueueRSHandle<'_, T> {
    fn push(&mut self, value: T) -> Result<(), T> {
        self.queue.lprqueue.enqueue(value);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.lprqueue.dequeue()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bq() {
        let q: LPRQRS<i32> = LPRQRS::new(100);
        q.lprqueue.push(1);
        assert_eq!(q.lprqueue.dequeue().unwrap(), 1);
    }
    #[test]
    fn register_bq() {
        let q: LPRQRS<i32> = LPRQRS::new(100);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let q: LPRQRS<i32> = LPRQRS::new(100);
        if crate::order::benchmark_order_i32(q, 10, 5, true, 10).is_err() {
            panic!();
        }
    }
}
