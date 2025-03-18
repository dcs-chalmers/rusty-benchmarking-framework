use crate::{ConcurrentQueue, Handle};

pub struct SCCQueue<T: 'static> {
    pub queue: scc::Queue<T>,
}

pub struct SCCQueueHandle<'a, T: 'static> {
    queue: &'a SCCQueue<T>
}

impl<T: Clone + Copy> ConcurrentQueue<T> for SCCQueue<T> {
    fn register(&self) -> impl Handle<T> {
        SCCQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("SCCQueue")
    }
    fn new(_size: usize) -> Self {
        SCCQueue {
            queue: scc::Queue::default()
        }
    }
}

impl<T: Clone + Copy> Handle<T> for SCCQueueHandle<'_, T> {
    fn push(&mut self, item: T)  -> Result<(), T> {
        let _ = self.queue.queue.push(item);
        Ok(())
    }
    
    fn pop(&mut self) -> Option<T> {
        self.queue.queue.pop().map(|e| **e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_scc_queue() {
        let q: SCCQueue<i32> = SCCQueue::new(1000);
        q.queue.push(1);
        assert_eq!(**q.queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_scc_queue() {
        let q: SCCQueue<i32> = SCCQueue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
