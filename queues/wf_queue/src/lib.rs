use wfqueue_upstream::{Queueable, WfQueue};
use benchmark_core::traits::{ConcurrentQueue, Handle};

pub struct WFQueueHandle<'a, T> {
    queue: &'a WFQueue<T>
}

pub struct WFQueue<T> {
    pub q: WfQueue<Box<T>>
}

impl<T> ConcurrentQueue<T> for WFQueue<T>
where
    Box<T>: Queueable
{
    fn register(&self) -> impl Handle<T> {
        WFQueueHandle {
            queue: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("wfqueue")
    }

    fn new(_size: usize) -> Self {
        WFQueue {
            q: WfQueue::new(_size)
        }
    }
}

impl<T> Handle<T> for WFQueueHandle<'_, T>
where
    Box<T>: Queueable
{
    fn push(&mut self, item: T) -> Result<(), T> {
        match self.queue.q.push(Box::new(item)) {
            Ok(()) => Ok(()),
            Err(boxed_item) => Err(*boxed_item),
        }
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.q.pop().map(|boxed_item| *boxed_item)
    }
}


#[cfg(test)]
mod tests {
    use super::{ConcurrentQueue, Handle, WFQueue};

    // #[test]
    // fn create_bq() {
    //     let q: WFQueue<Box<i32>> = WFQueue::new(1000);
    //     let _ = q.q.push(Box::new(32));
    //     assert_eq!(*q.q.pop().unwrap(), 32);
    // }
    #[test]
    fn register_bq() {
        let q: WFQueue<i32> = WFQueue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);
    }
    #[test]
    #[ignore]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: WFQueue<Box<i32>> = WFQueue::new(10);
        if benchmark_core::order::benchmark_order_box(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}
