use benchmark_core::traits::{ConcurrentQueue, HandleQueue};

pub struct LFQueue<T> {
    pub lfq: lf_queue_upstream::Queue<T>
}

pub struct LFQueueHandle<'a, T> {
    queue: &'a LFQueue<T>
}

impl<T> ConcurrentQueue<T> for LFQueue<T> {
    fn register(&self) -> impl HandleQueue<T> {
        LFQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("lf_queue")
    }
    fn new(_size: usize) -> Self {
        LFQueue {
            lfq: lf_queue_upstream::Queue::new()
        }
    }
}

impl<T> HandleQueue<T> for LFQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        self.queue.lfq.push(item);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.lfq.pop()
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lf_queue() {
        let q: LFQueue<i32> = LFQueue::new(1000);
        q.lfq.push(1);
        assert_eq!(q.lfq.pop().unwrap(), 1);
    }
    #[test]
    fn register_lf_queue() {
        let q: LFQueue<i32> = LFQueue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LFQueue<i32> = LFQueue::new(10);
        if benchmark_core::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}

