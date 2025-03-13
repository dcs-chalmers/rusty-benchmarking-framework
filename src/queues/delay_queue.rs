use std::time::Instant;

use crate::{ConcurrentQueue, Handle};
use delay_queue::{Delay, DelayQueue};

pub struct DQueue<T> {
    queue: DelayQueue<Delay<T>>
}
pub struct DQueueHandle<'a, T> {
    queue: &'a DQueue<T>
}

impl<T> ConcurrentQueue<T> for DQueue<T> {
    fn register(&self) -> impl Handle<T> {
        DQueueHandle {
            queue: self,
        } 
    }
    fn get_id(&self) -> String {
        String::from("DelayQueue")
    }
    fn new(size: usize) -> Self {
        DQueue {
            queue: DelayQueue::new()
        }
    }
}

impl<T> Handle<T> for DQueueHandle<'_, T> {
    fn pop(&mut self) -> Option<T> {
        Some(self.queue.queue.pop().value)
    }
    fn push(&mut self, item: T){
       self.queue.queue.push(Delay::until_instant(item, Instant::now())); 
    }
}
