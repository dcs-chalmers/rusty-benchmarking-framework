use crate::{ConcurrentQueue, Handle};
use std::sync::Arc;

pub struct ChuteQueue<T> {
    pub queue: Arc<chute::mpmc::Queue<T>>,
    reader: chute::mpmc::Reader<T>,
    writer: chute::mpmc::Writer<T>,
}

pub struct ChuteQueueHandle<'a, T> {
    queue: &'a ChuteQueue<T>
}

impl<T> ConcurrentQueue<T> for ChuteQueue<T> {
    fn register(&self) -> impl Handle<T> {
        ChuteQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        return String::from("ChuteQueue")
    }
    fn new(_size: usize) -> Self {
        let q = chute::mpmc::Queue::new();
        let r = q.reader();
        let writer = q.writer();
        ChuteQueue {
            queue: q,
            reader: r,
            writer
        }
    }
}

impl<T> Handle<T> for ChuteQueueHandle<'_, T> {
    fn push(&mut self, item: T) {
        self.queue.writer.push(item);
    }
    
    fn pop(&mut self) -> Option<T> {
        match self.queue.queue.pop() {
            Ok(val) => Some(val),
            Err(_) => None
        }
    }
}
