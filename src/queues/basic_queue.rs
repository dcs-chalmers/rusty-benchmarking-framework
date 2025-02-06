use crate::{ConcurrentQueue, Handle};
use std::{collections::VecDeque, sync::Mutex};

pub struct BQueue<T> {
    basic_queue: Mutex<VecDeque<T>>,
}

impl<T> BQueue<T> {
    pub fn pop(&self) -> Option<T> {
        let mut q = self.basic_queue.lock().unwrap();
        q.pop_front()
    }
    pub fn push(&self, item: T) {
        let mut q = self.basic_queue.lock().unwrap();
        q.push_back(item);
    }
    pub fn new() -> Self {
        BQueue {
            basic_queue: Mutex::new(VecDeque::new()),
        }
    }
}

pub struct BasicQueue<T> {
    pub bqueue: BQueue<T>
}

pub struct BasicQueueHandle<'a, T> {
    queue: &'a BasicQueue<T>
}

impl<T> ConcurrentQueue<T> for BasicQueue<T> {
    fn register(&self) -> impl Handle<T> {
        BasicQueueHandle {
            queue: self,
        }
    }
}

impl<T> Handle<T> for BasicQueueHandle<'_, T> {
    fn push(&mut self, item: T) {
        self.queue.bqueue.push(item);
    }
    fn pop(&mut self) -> Option<T> {
        self.queue.bqueue.pop()
    }
}


#[cfg(test)]
mod tests {
    use super::*;


    // Remove create tests?
    #[test]
    fn create_bq() {
        let q: BasicQueue<i32> = BasicQueue {
            bqueue: BQueue::new()
        };
        q.bqueue.push(1);
        assert_eq!(q.bqueue.pop().unwrap(), 1);
    }
    #[test]
    fn register_bq() {
        let q: BasicQueue<i32> = BasicQueue {
            bqueue: BQueue::new() 
        };
        let mut handle = q.register();
        handle.push(1);
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
