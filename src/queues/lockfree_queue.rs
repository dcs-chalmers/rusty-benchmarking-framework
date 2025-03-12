use crate::{ConcurrentQueue, Handle};


pub struct LockfreeQueueHandle<'a, T> {
    queue: &'a LockfreeQueue<T>
}

pub struct LockfreeQueue<T> {
    pub lfq: lockfree::queue::Queue<T>
}

impl<T> ConcurrentQueue<T> for LockfreeQueue<T> {
    fn register(&self) -> impl Handle<T> {
        LockfreeQueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        return String::from("Lockfree")
    }
    fn new(_size: usize) -> Self {
        LockfreeQueue {
            lfq: lockfree::queue::Queue::new(),
        }
    }
}

impl<T> Handle<T> for LockfreeQueueHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T>{
        self.queue.lfq.push(item);
        Ok(())
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
        let q: LockfreeQueue<i32> = LockfreeQueue {
            lfq: lockfree::queue::Queue::new()
        };
        q.lfq.push(1);
        assert_eq!(q.lfq.pop().unwrap(), 1);
    }
    #[test]
    fn register_lfq() {
        let q: LockfreeQueue<i32> = LockfreeQueue {
            lfq: lockfree::queue::Queue::new()
        };
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
