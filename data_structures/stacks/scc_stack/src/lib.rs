use benchmark_core::traits::{ConcurrentQueue, HandleQueue};

pub struct SCCStack<T: 'static> {
    pub queue: scc::Stack<T>,
}

pub struct SCCStackHandle<'a, T: 'static> {
    queue: &'a SCCStack<T>
}

impl<T: Clone + Copy> ConcurrentQueue<T> for SCCStack<T> {
    fn register(&self) -> impl HandleQueue<T> {
        SCCStackHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("scc_stack")
    }
    fn new(_size: usize) -> Self {
        SCCStack {
            queue: scc::Stack::default()
        }
    }
}

impl<T: Clone + Copy> HandleQueue<T> for SCCStackHandle<'_, T> {
    fn push(&mut self, item: T) -> Result<(), T> {
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
    fn create_scc_stack() {
        let q: SCCStack<i32> = SCCStack::new(1000);
        q.queue.push(1);
        assert_eq!(**q.queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_scc_stack() {
        let q: SCCStack<i32> = SCCStack::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
