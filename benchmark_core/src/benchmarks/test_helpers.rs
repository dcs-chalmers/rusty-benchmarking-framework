use crate::traits::{ConcurrentQueue, Handle};
use std::{collections::VecDeque, sync::Mutex};

/// A very simple ConcurrentQueue implementation for testing
mod test_queue {
    pub struct TestQueue<T> {
        queue: Mutex<VecDeque<T>>,
    }

    pub struct TestQueueHandle<'a, T> {
        queue: &'a TestQueue<T>,
    }

    impl<T> Handle<T> for TestQueueHandle<'_, T> {
        fn push(&mut self, item: T) -> Result<(), T>{
            self.queue.queue.lock().unwrap().push_back(item);
            Ok(())
        }

        fn pop(&mut self) -> Option<T> {
            self.queue.queue.lock().unwrap().pop_front()
        }
    }

    impl<T> ConcurrentQueue<T> for TestQueue<T> {
        fn register(&self) -> impl Handle<T> {
            TestQueueHandle {
                queue: self,
            }
        }

        fn get_id(&self) -> String {
            "test_queue".to_string()
        }

        fn new(_size: usize) -> Self {
            TestQueue {
                queue: Mutex::new(VecDeque::new())
            }
        }
    }
}
