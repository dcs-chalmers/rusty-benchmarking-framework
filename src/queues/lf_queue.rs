use crate::{ConcurrentQueue, Handle};

pub struct LFQueue<T> {
    pub lfq: lf_queue::Queue<T>
}

pub struct LFQueueHandle<'a, T> {
    queue: &'a LFQueue<T>
}

impl<T> ConcurrentQueue<T> for LFQueue<T> {
    fn register(&self) -> impl Handle<T> {
        LFQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        return String::from("LFQueue")
    }
    fn new(_size: usize) -> Self {
        LFQueue {
            lfq: lf_queue::Queue::new()
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lf_queue() {
        let q: LFQueue<i32> = LFQueue::new(1000);
        q.lfq.push(1);
        assert_eq!(q.lfq.pop().unwrap(), 1);
    }
    #[test]
    fn register_lf_queue() {
        let q: LFQueue<i32> = LFQueue::new(1000);
        let mut handle = q.register();
        handle.push(1);
        assert_eq!(handle.pop().unwrap(), 1);

    }
}

