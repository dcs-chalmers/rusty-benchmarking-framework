use benchmark_core::traits::{ConcurrentQueue, HandleQueue};
use bbq_upstream::BlockingQueue;

pub struct BBQueue<T>{
    pub queue: bbq_upstream::Bbq<T>,
}

pub struct BBQHandle<'a, T> {
    queue: &'a BBQueue<T>
}

impl<T: Default> ConcurrentQueue<T> for BBQueue <T>{
    fn register(&self) -> impl HandleQueue<T> {
        BBQHandle::<T> {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("bbq")
    }
    fn new(size: usize) -> Self {
        BBQueue {
            queue: bbq_upstream::Bbq::new(size, size).expect("Should never get here..."),
        }
    }
}

impl<T: Default> HandleQueue<T> for BBQHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T> {
        self.queue.queue.push(item).expect("failed push");
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
    fn create_bbq() {
        let q: BBQueue<i32> = BBQueue::new(100);
        let _ = q.queue.push(1);
        assert_eq!(q.queue.pop().unwrap(), 1);
    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: BBQueue<i32> = BBQueue::new(100000);
        if benchmark_core::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
