use crate::{ConcurrentQueue, Handle};
use crossbeam::queue::ArrayQueue;

pub struct AQueueHandle<'a, T>{
    queue: & 'a AQueue<T>
}

pub struct AQueue<T>{
    pub array_queue: ArrayQueue<T>
}


impl <T> ConcurrentQueue<T> for AQueue<T> {
    fn register(&self) -> impl Handle<T>{
        AQueueHandle{
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("ArrayQueue")
    }
    fn new(size: usize) -> Self {
        AQueue {
            array_queue: ArrayQueue::new(size),
        }
    }
}

impl<T> Handle<T> for AQueueHandle<'_, T>{
    fn push(&mut self, value: T) -> Result<(), T>{
        self.queue.array_queue.push(value)
    }

    fn pop(&mut self) -> Option<T> {
        self.queue.array_queue.pop()
    }
}
