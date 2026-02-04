use benchmark_core::traits::{ConcurrentPriorityQueue, HandlePriorityQueue};
use std::cmp::{Ordering, Reverse};
use std::{collections::binary_heap::BinaryHeap, sync::Mutex};

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

pub struct BinHeapWrap<P: Ord, T> {
    bin_heap: Mutex<BinaryHeap<KeyValuePair<P, T>>>,
}

impl<P: Ord, T> BinHeapWrap<P, T> {
    pub fn delete_min(&self) -> Option<KeyValuePair<P, T>> {
        let mut q = self.bin_heap.lock().unwrap();
        q.pop()
    }

    pub fn insert(&self, item: KeyValuePair<P, T>) {
        let mut q = self.bin_heap.lock().unwrap();
        q.push(item);
    }

    pub fn is_empty(&self) -> bool {
        self.bin_heap.lock().unwrap().is_empty()
    }

    pub fn new() -> Self {
        BinHeapWrap {
            bin_heap: Mutex::new(BinaryHeap::new()),
        }
    }
}

impl<P: Ord, T> Default for BinHeapWrap<P, T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BasicPriorityQueue<P: Ord, T> {
    pub basic_priority_queue: BinHeapWrap<P, T>,
}

pub struct BasicPriorityQueueHandle<'a, P: Ord, T> {
    priority_queue: &'a BasicPriorityQueue<P, T>,
}

impl<P: Ord, T> ConcurrentPriorityQueue<P, T> for BasicPriorityQueue<P, T> {
    fn register(&self) -> impl HandlePriorityQueue<P, T> {
        BasicPriorityQueueHandle {
            priority_queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("basic_priority_queue")
    }
    fn new(_size: usize) -> Self {
        BasicPriorityQueue {
            basic_priority_queue: BinHeapWrap::<P, T>::new(),
        }
    }
}

impl<P: Ord, T> HandlePriorityQueue<P, T>
    for BasicPriorityQueueHandle<'_, P, T>
{
    fn insert(&mut self, priority: P, item: T) -> Result<(), (P, T)> {
        let kv_pair = KeyValuePair::new(priority, item);
        self.priority_queue.basic_priority_queue.insert(kv_pair);
        Ok(())
    }
    fn delete_min(&mut self) -> Option<T> {
        match self.priority_queue.basic_priority_queue.delete_min() {
            Some(kv_pair) => Some(kv_pair.item),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_basic_priority_queue() {
        let pq: BasicPriorityQueue<i32, i32> = BasicPriorityQueue {
            basic_priority_queue: BinHeapWrap::new(),
        };
        pq.basic_priority_queue.insert(KeyValuePair::new(1, 10));
        assert_eq!(pq.basic_priority_queue.delete_min().unwrap().item, 10);
    }

    #[test]
    fn register_basic_priority_queue() {
        let pq = BasicPriorityQueue::<i32, i32>::new(0);
        let mut handle = pq.register();
        handle.insert(1, 10).unwrap();
        assert_eq!(handle.delete_min().unwrap(), 10);
    }

    #[test]
    fn test_order_basic_priority_queue() {
        let pq = BasicPriorityQueue::<i32, i32>::new(0);

        let mut handle = pq.register();
        assert_eq!(handle.insert(5, 50), Ok(()));
        assert_eq!(handle.insert(4, 40), Ok(()));
        assert_eq!(handle.insert(1, 10), Ok(()));
        assert_eq!(handle.insert(2, 20), Ok(()));
        assert_eq!(handle.insert(3, 30), Ok(()));

        assert_eq!(handle.delete_min(), Some(10));
        assert_eq!(handle.delete_min(), Some(20));
        assert_eq!(handle.delete_min(), Some(30));
        assert_eq!(handle.delete_min(), Some(40));
        assert_eq!(handle.delete_min(), Some(50));
        assert_eq!(handle.delete_min(), None);
    }
}
