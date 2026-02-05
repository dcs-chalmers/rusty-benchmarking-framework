/// A simple ConcurrentQueue implementation for testing
#[cfg(test)]
pub(crate) mod test_queue {
    use crate::traits::{ConcurrentQueue, HandleQueue};
    use std::collections::VecDeque;
    use std::sync::Mutex;

    pub struct TestQueue<T> {
        queue: Mutex<VecDeque<T>>,
    }

    pub struct TestQueueHandle<'a, T> {
        queue: &'a TestQueue<T>,
    }

    impl<T> HandleQueue<T> for TestQueueHandle<'_, T> {
        fn push(&mut self, item: T) -> Result<(), T> {
            self.queue.queue.lock().unwrap().push_back(item);
            Ok(())
        }

        fn pop(&mut self) -> Option<T> {
            self.queue.queue.lock().unwrap().pop_front()
        }
    }

    impl<T> ConcurrentQueue<T> for TestQueue<T> {
        fn register(&self) -> impl HandleQueue<T> {
            TestQueueHandle { queue: self }
        }

        fn get_id(&self) -> String {
            "test_queue".to_string()
        }

        fn new(_size: usize) -> Self {
            TestQueue {
                queue: Mutex::new(VecDeque::new()),
            }
        }
    }
}

/// A very simple ConcurrentPriorityQueue implementation for testing
#[cfg(test)]
pub(crate) mod test_priority_queue {
    use crate::traits::{ConcurrentPriorityQueue, HandlePriorityQueue};
    use std::cmp::{Ordering, Reverse};
    use std::collections::BinaryHeap;
    use std::sync::Mutex;

    pub struct KeyValuePair<P: Ord, T> {
        key: Reverse<P>,
        item: T,
    }

    impl<P: Ord, T> KeyValuePair<P, T> {
        pub fn new(key: P, item: T) -> Self {
            Self {
                key: Reverse(key),
                item,
            }
        }
    }

    impl<P: Ord, T> Ord for KeyValuePair<P, T> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.key.cmp(&other.key)
        }
    }

    impl<P: Ord, T> PartialOrd for KeyValuePair<P, T> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl<P: Ord, T> PartialEq for KeyValuePair<P, T> {
        fn eq(&self, other: &Self) -> bool {
            self.key == other.key
        }
    }

    impl<P: Ord, T> Eq for KeyValuePair<P, T> {}

    pub struct TestPriorityQueue<P: Ord, T> {
        queue: Mutex<BinaryHeap<KeyValuePair<P, T>>>,
    }

    pub struct TestPriorityQueueHandle<'a, P: Ord, T> {
        queue: &'a TestPriorityQueue<P, T>,
    }

    impl<P: Ord, T> HandlePriorityQueue<P, T>
        for TestPriorityQueueHandle<'_, P, T>
    {
        fn insert(&mut self, priority: P, item: T) -> Result<(), (P, T)> {
            let kv_pair = KeyValuePair::new(priority, item);
            self.queue.queue.lock().unwrap().push(kv_pair);
            Ok(())
        }
        fn delete_min(&mut self) -> Option<T> {
            match self.queue.queue.lock().unwrap().pop() {
                Some(kv_pair) => Some(kv_pair.item),
                None => None,
            }
        }
        fn is_empty(&mut self) -> bool {
            self.queue.queue.lock().unwrap().is_empty()
        }
    }

    impl<P: Ord, T> ConcurrentPriorityQueue<P, T> for TestPriorityQueue<P, T> {
        fn register(&self) -> impl HandlePriorityQueue<P, T> {
            TestPriorityQueueHandle { queue: self }
        }

        fn get_id(&self) -> String {
            "test_queue".to_string()
        }

        fn new(_size: usize) -> Self {
            TestPriorityQueue {
                queue: Mutex::new(BinaryHeap::new()),
            }
        }
    }
}
