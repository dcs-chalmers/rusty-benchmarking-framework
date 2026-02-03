use benchmark_core::traits::{ConcurrentQueue, HandleQueue};
use crossbeam::queue::ArrayQueue;

pub struct AQueueHandle<'a, T>{
    queue: & 'a AQueue<T>
}

pub struct AQueue<T>{
    pub array_queue: ArrayQueue<T>
}


impl <T> ConcurrentQueue<T> for AQueue<T> {
    fn register(&self) -> impl HandleQueue<T>{
        AQueueHandle{
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("array_queue")
    }
    fn new(size: usize) -> Self {
        AQueue {
            array_queue: ArrayQueue::new(size),
        }
    }
}

impl<T> HandleQueue<T> for AQueueHandle<'_, T>{
    fn push(&mut self, value: T) -> Result<(), T>{
        self.queue.array_queue.push(value)
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.array_queue.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Remove create tests?
    #[test]
    fn create_bq() {
        let q: AQueue<i32> = AQueue {
            array_queue: ArrayQueue::new(100)
        };
        let _ = q.array_queue.push(1);
        assert_eq!(q.array_queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_bq() {
        let q: AQueue<i32> = AQueue {
            array_queue: ArrayQueue::new(100)
        };
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let q: AQueue<i32> = AQueue {
            array_queue: ArrayQueue::new(10)
        };
        if benchmark_core::order::benchmark_order_i32(q, 10, 5, true, 10).is_err() {
            panic!();
        }
    }
}
