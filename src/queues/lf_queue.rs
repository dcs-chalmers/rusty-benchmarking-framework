use crate::{ConcurrentQueue, Handle};


pub struct LFQueueHandle<'a, T> {
    queue: &'a LFQueue<T>
}

pub struct LFQueue<T> {
    pub lfq: lockfree::queue::Queue<T>
}

impl<T> ConcurrentQueue<T> for LFQueue<T> {
    
    fn register(&self) -> impl Handle<T> {
        LFQueueHandle {
            queue: self,
        }
    }
}

impl<T> Handle<T> for LFQueueHandle<'_, T> {
    fn push(&mut self, item: T) {
        self.queue.lfq.push(item);
    }
    
    fn pop(&mut self) -> Option<T> {
        self.queue.lfq.pop()
    }

}
