use crate::traits::{ConcurrentQueue, Handle};

pub struct SCC2Queue<T: 'static> {
    pub queue: scc2::Queue<T>,
}

pub struct SCC2QueueHandle<'a, T: 'static> {
    queue: &'a SCC2Queue<T>
}

impl<T: Clone + Copy> ConcurrentQueue<T> for SCC2Queue<T> {
    fn register(&self) -> impl Handle<T> {
        SCC2QueueHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("scc2_queue")
    }
    fn new(_size: usize) -> Self {
        SCC2Queue {
            queue: scc2::Queue::default()
        }
    }
}

impl<T: Clone + Copy> Handle<T> for SCC2QueueHandle<'_, T> {
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
    fn create_scc2_queue() {
        let q: SCC2Queue<i32> = SCC2Queue::new(1000);
        q.queue.push(1);
        assert_eq!(**q.queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_scc2_queue() {
        let q: SCC2Queue<i32> = SCC2Queue::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
    #[test]
    #[ignore]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: SCC2Queue<i32> = SCC2Queue::new(10);
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
}

