use crate::{ConcurrentQueue, Handle};
use crossbeam::queue::SegQueue;

pub struct SegQueueHandle<'a, T>{
    queue: & 'a SQueue<T>
}

pub struct SQueue<T>{
    pub seg_queue: SegQueue<T>
}

impl <T> ConcurrentQueue<T> for SQueue<T>{
    fn register(&self) -> impl Handle<T>{
        SegQueueHandle{
            queue: self,
        }
    }

    fn get_id(&self) -> String {
        return String::from("SegQueue")
    }

    fn new(_size: usize) -> Self {
        SQueue {
            seg_queue: SegQueue::new(),
        }
    }
}

impl <T> Handle<T> for SegQueueHandle<'_, T> {
    fn push(&mut self, value: T) -> Result<(), T> {
        self.queue.seg_queue.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.seg_queue.pop()
    }
}
