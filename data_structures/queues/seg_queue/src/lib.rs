use benchmark_core::traits::{ConcurrentQueue, HandleQueue};
use crossbeam::queue::SegQueue;

pub struct SegQueueHandle<'a, T>{
    queue: & 'a SQueue<T>
}

pub struct SQueue<T>{
    pub seg_queue: SegQueue<T>
}

impl <T> ConcurrentQueue<T> for SQueue<T>{
    fn register(&self) -> impl HandleQueue<T>{
        SegQueueHandle{
            queue: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("seg_queue")
    }

    fn new(_size: usize) -> Self {
        SQueue {
            seg_queue: SegQueue::new(),
        }
    }
}

impl <T> HandleQueue<T> for SegQueueHandle<'_, T> {
    fn push(&mut self, value: T) -> Result<(), T> {
        self.queue.seg_queue.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.seg_queue.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_bq() {
        let q: SQueue<i32> = SQueue::new(100);
        q.seg_queue.push(1);
        assert_eq!(q.seg_queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_bq() {
        let q: SQueue<i32> = SQueue::new(100);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let q: SQueue<i32> = SQueue::new(100);
        if benchmark_core::order::benchmark_order_i32(q, 10, 5, true, 10).is_err() {
            panic!();
        }
    }
}
