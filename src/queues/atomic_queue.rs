use crate::traits::{ConcurrentQueue, Handle};

pub struct AtomicQueue<T> {
    pub queue: atomic_queue::Queue<T>,
}

pub struct AtomicQueueHandle<'a, T> {
    queue: &'a AtomicQueue<T>
}

impl<T: Default> ConcurrentQueue<T> for AtomicQueue<T> {
    fn register(&self) -> impl Handle<T> {
        AtomicQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("atomic_queue")
    }
    fn new(size: usize) -> Self {
        AtomicQueue {
            queue: atomic_queue::bounded(size),
        }
    }
}

impl<T: Default> Handle<T> for AtomicQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        if !self.queue.queue.push(item) {
            return Err(T::default());
        }
        Ok(())
    }
    
    fn pop(&mut self) -> Option<T> {
        self.queue.queue.pop()
    }
}


 
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_atomic_queue() {
        let q: AtomicQueue<i32> = AtomicQueue::new(1000);
        q.queue.push(1);
        assert_eq!(q.queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_atomic_queue() {
        let q: AtomicQueue<i32> = AtomicQueue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: AtomicQueue<i32> = AtomicQueue::new(10);
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }

}
