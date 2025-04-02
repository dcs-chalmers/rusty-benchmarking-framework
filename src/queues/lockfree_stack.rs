// now get to work jam
use crate::{ConcurrentQueue, Handle};
pub struct LockfreeStack<T>{
    pub lfs: lockfree::stack::Stack<T>
}

pub struct LockFreeStacKHandle<'a, T> {
    stack: & 'a LockfreeStack<T>
}

impl<T> ConcurrentQueue<T> for LockfreeStack<T> {
    fn register(&self) -> impl Handle<T> {
        LockFreeStacKHandle{
            stack: self,
        }
    }
    fn get_id(&self) -> String {
        String::from("lockfree_stack")
    }
    fn new(_size: usize) -> Self {
        LockfreeStack{
            lfs: lockfree::stack::Stack::new()
        }
    }
}

impl<T> Handle<T> for LockFreeStacKHandle<'_, T>{
    fn push(&mut self, item: T) -> Result<(), T>{
        self.stack.lfs.push(item);
        Ok(())
    }
    fn pop(&mut self) -> Option<T> {
        self.stack.lfs.pop()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lockfree_stack() {
        let stack = LockfreeStack::new(0);
        stack.lfs.push(1);
        assert_eq!(stack.lfs.pop().unwrap(), 1);
    }
    #[test]
    fn register_lockfree_stack() {
        let stack: LockfreeStack<i32> = LockfreeStack {
            lfs: lockfree::stack::Stack::new()
        };
        let mut handle = stack.register();
        handle.push(1).unwrap();
        assert_eq!(handle.pop().unwrap(), 1);

    }
}
