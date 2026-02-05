use benchmark_core::traits::{ConcurrentQueue, HandleQueue};
use lprq_rs::LPRQueue;

pub struct LPRQueueRSHandle<'a, T>{
    queue: & 'a LPRQRS< T>
}

pub struct LPRQRS<T>{
    pub lprqueue: LPRQueue<T>
}

impl <T> ConcurrentQueue<T> for LPRQRS<T>{
    fn register(&self) -> impl HandleQueue<T>{
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

impl <T> HandleQueue<T> for LPRQueueRSHandle<'_, T> {
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
        q.lprqueue.enqueue(1);
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
    #[ignore]
    fn test_order() {
        let q: LPRQRS<i32> = LPRQRS::new(100);
        if benchmark_core::order::benchmark_order_i32(q, 10, 5, true, 10).is_err() {
            panic!();
        }
    }
}
