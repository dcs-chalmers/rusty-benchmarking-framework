use wfqueue::{Queueable, WfQueue};

use crate::{ConcurrentQueue, Handle};


pub struct WFQueueHandle<'a, T: Queueable> {
    queue: &'a WFQueue<T>
}

pub struct WFQueue<T: Queueable> {
    pub q: WfQueue<T>
}

impl<T: Queueable> ConcurrentQueue<T> for WFQueue<T> {
    fn register(&self) -> impl Handle<T> {
        WFQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        return String::from("WFQueue")
    }
    fn new(_size: usize) -> Self {
        WFQueue {
            q: WfQueue::new(_size)
        }
    }
}

impl<T: Queueable> Handle<T> for WFQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T> {
        self.queue.q.push(item)
    }
    
    fn pop(&mut self) -> Option<T> {
        self.queue.q.pop()
    }

}


#[cfg(test)]
mod tests {
    use super::{ConcurrentQueue, Handle, WFQueue};

    #[test]
    fn create_bq() {
        let q: WFQueue<Box<i32>> = WFQueue::new(1000);
        let _ = q.q.push(Box::new(32));
        assert_eq!(*q.q.pop().unwrap(), 32);
    }
    #[test]
    fn register_bq() {
        let q: WFQueue<Box<i32>> = WFQueue::new(1000);
        let mut handle = q.register();
        handle.push(Box::new(1)).unwrap();
        assert_eq!(*handle.pop().unwrap(), 1);
    }
}
