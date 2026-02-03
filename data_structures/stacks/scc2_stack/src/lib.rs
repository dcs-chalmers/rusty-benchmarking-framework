use benchmark_core::traits::{ConcurrentQueue, HandleQueue};

pub struct SCC2Stack<T: 'static> {
    pub queue: scc2::Stack<T>,
}

pub struct SCC2StackHandle<'a, T: 'static> {
    queue: &'a SCC2Stack<T>
}

impl<T: Clone + Copy> ConcurrentQueue<T> for SCC2Stack<T> {
    fn register(&self) -> impl HandleQueue<T> {
        SCC2StackHandle {
            queue: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("scc2_stack")
    }
    fn new(_size: usize) -> Self {
        SCC2Stack {
            queue: scc2::Stack::default()
        }
    }
}

impl<T: Clone + Copy> HandleQueue<T> for SCC2StackHandle<'_, T> {
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
    fn create_scc2_stack() {
        let q: SCC2Stack<i32> = SCC2Stack::new(1000);
        q.queue.push(1);
        assert_eq!(**q.queue.pop().unwrap(), 1);
    }
    #[test]
    fn register_scc2_stack() {
        let q: SCC2Stack<i32> = SCC2Stack::new(1000);
        let mut handle = q.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
