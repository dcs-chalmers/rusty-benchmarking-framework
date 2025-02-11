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
    fn get_id(&self) -> String {
        return String::from("Lockfree")
    }
    fn new(_size: usize) -> Self {
        LFQueue {
            lfq: lockfree::queue::Queue::new(),
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
    fn create_lfq() {
        let q: LFQueue<i32> = LFQueue {
            lfq: lockfree::queue::Queue::new()
        };
        q.lfq.push(1);
        assert_eq!(q.lfq.pop().unwrap(), 1);
    }
    #[test]
    fn register_lfq() {
        let q: LFQueue<i32> = LFQueue {
            lfq: lockfree::queue::Queue::new()
        };
        let mut handle = q.register();
        handle.push(1);
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
